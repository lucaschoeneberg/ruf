/* Deterministic stalled-service regression. No real notification service or consent. */
#define SIP_NOTIFICATION_SERVICE_TIMEOUT_NS (20 * NSEC_PER_MSEC)
#include "../call_notifications.m"
#include <stdio.h>

@interface TestSettings : NSObject
@property(nonatomic) UNAuthorizationStatus status;
@end
@implementation TestSettings
- (UNAuthorizationStatus)authorizationStatus { return self.status; }
@end

@interface StalledCenter : NSObject
@property(nonatomic) BOOL stallSettings;
@property(nonatomic) BOOL denied;
@property(nonatomic) BOOL failDelivery;
@property(nonatomic) int permissionRequests;
@property(nonatomic, strong) UNNotificationRequest *lastRequest;
@property(nonatomic) BOOL lateSettings;
@property(nonatomic) BOOL lateDelivery;
@property(nonatomic) BOOL acknowledgeDelivery;
@property(atomic, copy) NSString *visibleToken;
@property(nonatomic, copy) dispatch_block_t pendingSettings;
@property(nonatomic, copy) dispatch_block_t pendingDelivery;
@property(nonatomic) int removed;
- (void)finishSettings;
- (void)finishDelivery;
@end
@implementation StalledCenter
- (void)getNotificationSettingsWithCompletionHandler:(void (^)(UNNotificationSettings *))completion {
    if (self.stallSettings) return;
    TestSettings *settings = [[TestSettings alloc] init];
    settings.status = self.denied ? UNAuthorizationStatusDenied : UNAuthorizationStatusAuthorized;
    if (self.lateSettings) {
        self.pendingSettings = ^{ completion((UNNotificationSettings *)(id)settings); };
        return;
    }
    completion((UNNotificationSettings *)(id)settings);
}
- (void)requestAuthorizationWithOptions:(UNAuthorizationOptions)options completionHandler:(void (^)(BOOL, NSError *))completion {
    (void)options;
    self.permissionRequests++;
    completion(NO, nil);
}
- (void)addNotificationRequest:(UNNotificationRequest *)request withCompletionHandler:(void (^)(NSError *))completion {
    self.lastRequest = request;
    if (self.failDelivery) {
        completion([NSError errorWithDomain:@"synthetic" code:1 userInfo:nil]);
    } else if (self.lateDelivery) {
        __weak StalledCenter *weakSelf = self;
        self.pendingDelivery = ^{
            weakSelf.visibleToken = request.identifier;
            completion(nil);
        };
    } else if (self.acknowledgeDelivery) {
        self.visibleToken = request.identifier;
        completion(nil);
    } /* Otherwise simulate a daemon that never acknowledges. */
}
- (void)finishSettings {
    dispatch_block_t completion = self.pendingSettings;
    self.pendingSettings = nil;
    if (!completion) exit(29);
    completion();
}
- (void)finishDelivery {
    dispatch_block_t completion = self.pendingDelivery;
    self.pendingDelivery = nil;
    if (!completion) exit(30);
    completion();
}
- (void)removePendingNotificationRequestsWithIdentifiers:(NSArray<NSString *> *)tokens {
    (void)tokens;
    self.removed++;
}
- (void)removeDeliveredNotificationsWithIdentifiers:(NSArray<NSString *> *)tokens {
    if ([tokens containsObject:self.visibleToken]) self.visibleToken = nil;
    self.removed++;
}
@end

@interface TestNotification : NSObject
@property(nonatomic, strong) UNNotificationRequest *request;
@end
@implementation TestNotification
@end
@interface TestResponse : NSObject
@property(nonatomic, copy) NSString *actionIdentifier;
@property(nonatomic, strong) TestNotification *notification;
@end
@implementation TestResponse
@end

static int action_count;
static void record_action(int action, const char *token) {
    if (action != 3 || strcmp(token, "live-call")) exit(23);
    action_count++;
}

int main(void) {
    @autoreleasepool {
        StalledCenter *fake = [[StalledCenter alloc] init];
        center = (UNUserNotificationCenter *)(id)fake;
        state_lock = [[NSObject alloc] init];
        delivery_queue = dispatch_queue_create("test.notification-timeouts", DISPATCH_QUEUE_SERIAL);
        dispatch_semaphore_t finished = dispatch_semaphore_create(0);
        dispatch_async(dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0), ^{
            int permission = 3;
            fake.stallSettings = YES;
            if (!sip_call_notifications_permission(0, &permission) || permission != 3) exit(1);
            fake.stallSettings = NO;
            fake.lateSettings = YES;
            if (!sip_call_notifications_permission(0, &permission) || permission != 3) exit(2);
            /* Release only after timeout, regardless of worker scheduling. */
            [fake finishSettings];
            if (permission != 3) exit(31);
            fake.lateSettings = NO;
            if (sip_call_notifications_permission(0, &permission) || permission != 2) exit(3);
            if (!sip_call_notifications_show("test-token", "Test", "Synthetic call")) exit(4);
            if (sip_call_notifications_dismiss("test-token") || fake.removed != 2) exit(5);
            fake.lateDelivery = YES;
            if (!sip_call_notifications_show("late-token", "Test", "Synthetic call")) exit(6);
            if (sip_call_notifications_dismiss("late-token")) exit(7);
            [fake finishDelivery];
            dispatch_sync(delivery_queue, ^{});
            if (fake.visibleToken) exit(8);
            if (!sip_call_notifications_show("retry-token", "Test", "Synthetic call")) exit(9);
            fake.lateDelivery = NO;
            fake.acknowledgeDelivery = YES;
            if (sip_call_notifications_show("retry-token", "Test", "Synthetic call")) exit(10);
            [fake finishDelivery];
            dispatch_sync(delivery_queue, ^{});
            if (![fake.visibleToken isEqualToString:@"retry-token"]) exit(11);
            if (sip_call_notifications_dismiss("retry-token")) exit(12);
            /* The settings test has no call token/category/personal payload and
             * may not wait behind real-call delivery or touch its withdrawal state. */
            active_token = @"live-call";
            int removed = fake.removed;
            dispatch_semaphore_t releaseQueue = dispatch_semaphore_create(0);
            dispatch_async(delivery_queue, ^{ dispatch_semaphore_wait(releaseQueue, DISPATCH_TIME_FOREVER); });
            if (sip_test_notification_show()) exit(16);
            if (![fake.lastRequest.identifier hasPrefix:@"org.sipsoftphone.settings-test."] ||
                fake.lastRequest.content.categoryIdentifier.length || fake.lastRequest.content.userInfo.count ||
                fake.lastRequest.content.sound ||
                ![fake.lastRequest.content.title isEqualToString:@"Testbenachrichtigung"] ||
                ![fake.lastRequest.content.body isEqualToString:@"Benachrichtigungen von Ruf sind aktiviert."] ||
                ![active_token isEqualToString:@"live-call"] || fake.removed != removed) exit(17);
            SIPCallNotificationDelegate *delegate = [[SIPCallNotificationDelegate alloc] init];
            action_callback = record_action;
            TestResponse *response = [[TestResponse alloc] init];
            response.notification = [[TestNotification alloc] init];
            response.notification.request = fake.lastRequest;
            for (NSString *action in @[UNNotificationDefaultActionIdentifier, accept_id, reject_id]) {
                response.actionIdentifier = action;
                __block BOOL completed = NO;
                [delegate userNotificationCenter:center didReceiveNotificationResponse:(UNNotificationResponse *)(id)response
                    withCompletionHandler:^{ completed = YES; }];
                if (!completed || action_count) exit(24);
            }
            response.actionIdentifier = UNNotificationDefaultActionIdentifier;
            response.notification.request = [UNNotificationRequest requestWithIdentifier:@"live-call"
                content:fake.lastRequest.content trigger:nil];
            [delegate userNotificationCenter:center didReceiveNotificationResponse:(UNNotificationResponse *)(id)response
                withCompletionHandler:^{}];
            if (action_count != 1) exit(25);
            dispatch_semaphore_signal(releaseQueue);
            fake.denied = YES;
            fake.lastRequest = nil;
            if (!sip_test_notification_show() || fake.lastRequest || fake.permissionRequests) exit(18);
            fake.denied = NO;
            fake.acknowledgeDelivery = NO;
            if (!sip_test_notification_show()) exit(19);
            fake.lateDelivery = YES;
            if (!sip_test_notification_show()) exit(20);
            [fake finishDelivery];
            dispatch_sync(delivery_queue, ^{});
            if (![active_token isEqualToString:@"live-call"] || fake.removed != removed) exit(22);
            fake.lateDelivery = NO;
            fake.stallSettings = YES;
            fake.lastRequest = nil;
            if (!sip_test_notification_show() || fake.lastRequest) exit(26);
            fake.stallSettings = NO;
            fake.lateSettings = YES;
            if (!sip_test_notification_show() || fake.lastRequest) exit(27);
            fake.lateSettings = NO;
            fake.failDelivery = YES;
            if (!sip_test_notification_show() || fake.permissionRequests) exit(28);
            /* Late settings callbacks must be safe after their caller timed out. */
            [fake finishSettings];
            dispatch_semaphore_signal(finished);
        });
        dispatch_semaphore_wait(finished, DISPATCH_TIME_FOREVER);
        puts("MACOS_NOTIFICATION_TIMEOUTS_OK: stalled settings/delivery, late callbacks, withdrawal, live retry, anonymous independent test delivery");
    }
    return 0;
}
