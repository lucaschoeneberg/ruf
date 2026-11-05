/* Deterministic stalled-service regression. No real notification service or consent. */
#define SIP_NOTIFICATION_SERVICE_TIMEOUT_NS (20 * NSEC_PER_MSEC)
#include "../call_notifications.m"
#include <stdio.h>

@interface TestSettings : NSObject
@end
@implementation TestSettings
- (UNAuthorizationStatus)authorizationStatus { return UNAuthorizationStatusAuthorized; }
@end

@interface StalledCenter : NSObject
@property(nonatomic) BOOL stallSettings;
@property(nonatomic) BOOL lateSettings;
@property(nonatomic) BOOL lateDelivery;
@property(nonatomic) BOOL acknowledgeDelivery;
@property(atomic, copy) NSString *visibleToken;
@property(nonatomic, strong) dispatch_semaphore_t lateCompleted;
@property(nonatomic) int removed;
@end
@implementation StalledCenter
- (void)getNotificationSettingsWithCompletionHandler:(void (^)(UNNotificationSettings *))completion {
    if (self.stallSettings) return;
    if (self.lateSettings) {
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 50 * NSEC_PER_MSEC),
            dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0), ^{
                completion((UNNotificationSettings *)(id)[[TestSettings alloc] init]);
            });
        return;
    }
    completion((UNNotificationSettings *)(id)[[TestSettings alloc] init]);
}
- (void)addNotificationRequest:(UNNotificationRequest *)request withCompletionHandler:(void (^)(NSError *))completion {
    if (self.lateDelivery) {
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 60 * NSEC_PER_MSEC),
            dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0), ^{
                self.visibleToken = request.identifier;
                completion(nil);
                dispatch_semaphore_signal(self.lateCompleted);
            });
    } else if (self.acknowledgeDelivery) {
        self.visibleToken = request.identifier;
        completion(nil);
    } /* Otherwise simulate a daemon that never acknowledges. */
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

int main(void) {
    @autoreleasepool {
        StalledCenter *fake = [[StalledCenter alloc] init];
        fake.lateCompleted = dispatch_semaphore_create(0);
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
            fake.lateSettings = NO;
            if (sip_call_notifications_permission(0, &permission) || permission != 2) exit(3);
            if (!sip_call_notifications_show("test-token", "Test", "Synthetic call")) exit(4);
            if (sip_call_notifications_dismiss("test-token") || fake.removed != 2) exit(5);
            fake.lateDelivery = YES;
            if (!sip_call_notifications_show("late-token", "Test", "Synthetic call")) exit(6);
            if (sip_call_notifications_dismiss("late-token")) exit(7);
            if (dispatch_semaphore_wait(fake.lateCompleted, dispatch_time(DISPATCH_TIME_NOW, NSEC_PER_SEC))) exit(13);
            dispatch_sync(delivery_queue, ^{});
            if (fake.visibleToken) exit(8);
            if (!sip_call_notifications_show("retry-token", "Test", "Synthetic call")) exit(9);
            fake.lateDelivery = NO;
            fake.acknowledgeDelivery = YES;
            if (sip_call_notifications_show("retry-token", "Test", "Synthetic call")) exit(10);
            if (dispatch_semaphore_wait(fake.lateCompleted, dispatch_time(DISPATCH_TIME_NOW, NSEC_PER_SEC))) exit(14);
            dispatch_sync(delivery_queue, ^{});
            if (![fake.visibleToken isEqualToString:@"retry-token"]) exit(11);
            if (sip_call_notifications_dismiss("retry-token")) exit(12);
            /* Late settings callbacks must be safe after their caller timed out. */
            [NSThread sleepForTimeInterval:0.1];
            dispatch_semaphore_signal(finished);
        });
        dispatch_semaphore_wait(finished, DISPATCH_TIME_FOREVER);
        puts("MACOS_NOTIFICATION_TIMEOUTS_OK: stalled settings/delivery, late callbacks, withdrawal, live retry");
    }
    return 0;
}
