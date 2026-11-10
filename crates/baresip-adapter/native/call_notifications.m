#import <Foundation/Foundation.h>
#import <UserNotifications/UserNotifications.h>
#import <dispatch/dispatch.h>
#include "call_notifications.h"
#include <string.h>

/* A stalled notification daemon must not retain the delivery worker forever. */
#ifndef SIP_NOTIFICATION_SERVICE_TIMEOUT_NS
#define SIP_NOTIFICATION_SERVICE_TIMEOUT_NS (5 * NSEC_PER_SEC)
#endif
#define SIP_NOTIFICATION_AUTHORIZATION_TIMEOUT_NS (180 * NSEC_PER_SEC)

static NSString *const test_identifier_prefix = @"org.sipsoftphone.settings-test.";
static NSString *const category_id = @"org.sipsoftphone.incoming";
static NSString *const accept_id = @"accept";
static NSString *const reject_id = @"reject";
static UNUserNotificationCenter *center;
static dispatch_queue_t delivery_queue;
static NSObject *state_lock;
static NSString *active_token;
static sip_call_notification_callback action_callback;

static NSString *token_value(const char *bytes) {
    if (!bytes) return nil;
    size_t length = strnlen(bytes, 129);
    if (!length || length > 128) return nil;
    for (size_t i = 0; i < length; i++) {
        unsigned char c = (unsigned char)bytes[i];
        if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
              (c >= '0' && c <= '9') || c == '-' || c == '_' || c == '.')) return nil;
    }
    return [[NSString alloc] initWithBytes:bytes length:length encoding:NSUTF8StringEncoding];
}

@interface SIPCallNotificationDelegate : NSObject <UNUserNotificationCenterDelegate>
@end

@implementation SIPCallNotificationDelegate
- (void)userNotificationCenter:(UNUserNotificationCenter *)notificationCenter
       willPresentNotification:(UNNotification *)notification
         withCompletionHandler:(void (^)(UNNotificationPresentationOptions))completionHandler {
    (void)notificationCenter;
    (void)notification;
    completionHandler(UNNotificationPresentationOptionBanner | UNNotificationPresentationOptionList);
}

- (void)userNotificationCenter:(UNUserNotificationCenter *)notificationCenter
didReceiveNotificationResponse:(UNNotificationResponse *)response
         withCompletionHandler:(void (^)(void))completionHandler {
    (void)notificationCenter;
    @try {
        int action = 0;
        if ([response.actionIdentifier isEqualToString:accept_id]) action = 1;
        else if ([response.actionIdentifier isEqualToString:reject_id]) action = 2;
        else if ([response.actionIdentifier isEqualToString:UNNotificationDefaultActionIdentifier]) action = 3;
        NSString *token = response.notification.request.identifier;
        const char *bytes = token.UTF8String;
        if (![token hasPrefix:test_identifier_prefix] && action && action_callback && token_value(bytes) &&
            strlen(bytes) == [token lengthOfBytesUsingEncoding:NSUTF8StringEncoding]) {
            /* Rust copies the opaque token immediately and returns without executing SIP. */
            action_callback(action, bytes);
        }
    } @catch (NSException *exception) {
        (void)exception; /* Never expose native exception details or caller information. */
    } @finally {
        completionHandler();
    }
}
@end

static SIPCallNotificationDelegate *notification_delegate;

int sip_call_notifications_init(const char *app_id, sip_call_notification_callback callback) {
    if (!app_id || !callback) return 1;
    @autoreleasepool {
        @try {
            NSBundle *bundle = NSBundle.mainBundle;
            NSString *identifier = [[NSString alloc] initWithUTF8String:app_id];
            if (!identifier.length || ![bundle.bundleIdentifier isEqualToString:identifier] ||
                ![bundle.bundleURL.pathExtension isEqualToString:@"app"]) return 1;
            static dispatch_once_t once;
            dispatch_once(&once, ^{
                state_lock = [[NSObject alloc] init];
                delivery_queue = dispatch_queue_create("org.sipsoftphone.call-notifications", DISPATCH_QUEUE_SERIAL);
                action_callback = callback;
                notification_delegate = [[SIPCallNotificationDelegate alloc] init];
                center = UNUserNotificationCenter.currentNotificationCenter;
                center.delegate = notification_delegate;

            });
            return center && notification_delegate ? 0 : 1;
        } @catch (NSException *exception) {
            (void)exception;
            return 1;
        }
    }
}

static int permission_value(UNAuthorizationStatus status) {
    switch (status) {
        case UNAuthorizationStatusNotDetermined: return 0;
        case UNAuthorizationStatusDenied: return 1;
        case UNAuthorizationStatusAuthorized:
        case UNAuthorizationStatusProvisional: return 2;
        default: return 1;
    }
}

static int read_permission(int *permission) {
    dispatch_semaphore_t completed = dispatch_semaphore_create(0);
    __block int result = 1;
    [center getNotificationSettingsWithCompletionHandler:^(UNNotificationSettings *settings) {
        result = permission_value(settings.authorizationStatus);
        dispatch_semaphore_signal(completed);
    }];
    if (dispatch_semaphore_wait(completed,
            dispatch_time(DISPATCH_TIME_NOW, SIP_NOTIFICATION_SERVICE_TIMEOUT_NS))) return 1;
    *permission = result;
    return 0;
}

int sip_call_notifications_permission(int request, int *permission) {
    if (!permission || NSThread.isMainThread) return 1;
    *permission = 3;
    if (!center) return 0;
    @autoreleasepool {
        @try {
            if (read_permission(permission)) return 1;
            if (!request || *permission != 0) return 0;
            dispatch_semaphore_t completed = dispatch_semaphore_create(0);
            __block BOOL failed = NO;
            [center requestAuthorizationWithOptions:UNAuthorizationOptionAlert completionHandler:^(BOOL granted, NSError *error) {
                (void)granted;
                failed = error != nil;
                dispatch_semaphore_signal(completed);
            }];
            if (dispatch_semaphore_wait(completed,
                    dispatch_time(DISPATCH_TIME_NOW, SIP_NOTIFICATION_AUTHORIZATION_TIMEOUT_NS))) return 1;
            if (failed) return 1;
            return read_permission(permission);
        } @catch (NSException *exception) {
            (void)exception;
            return 1;
        }
    }
}

int sip_call_notifications_show(const char *token_bytes, const char *title_bytes, const char *body_bytes, const char *answer_bytes, const char *reject_bytes) {
    if (!center || NSThread.isMainThread || !title_bytes || !body_bytes || !answer_bytes || !reject_bytes) return 1;
    @autoreleasepool {
        @try {
            NSString *token = token_value(token_bytes);
            NSString *title = [[NSString alloc] initWithUTF8String:title_bytes];
            NSString *body = [[NSString alloc] initWithUTF8String:body_bytes];
            NSString *answer = [[NSString alloc] initWithUTF8String:answer_bytes];
            NSString *reject = [[NSString alloc] initWithUTF8String:reject_bytes];
            if (!token || !title.length || !body.length || !answer.length || !reject.length) return 1;
            /* Replace categories on every delivery, including live language changes.
             * dispatch_once owns the delegate, never the localized labels. */
            NSArray<UNNotificationAction *> *actions = @[
                [UNNotificationAction actionWithIdentifier:accept_id title:answer options:0],
                [UNNotificationAction actionWithIdentifier:reject_id title:reject options:0]
            ];
            UNNotificationCategory *category = [UNNotificationCategory categoryWithIdentifier:category_id
                actions:actions intentIdentifiers:@[] options:0];
            [center setNotificationCategories:[NSSet setWithObject:category]];
            @synchronized(state_lock) { active_token = token; }
            int permission;
            if (read_permission(&permission) || permission != 2) return 1;
            __block int result = 0;
            dispatch_sync(delivery_queue, ^{
                @autoreleasepool {
                    @try {
                        dispatch_semaphore_t completed = dispatch_semaphore_create(0);
                        __block BOOL failed = NO;
                        @synchronized(state_lock) {
                            /* A withdrawal during the settings query cancels delivery. */
                            if (![active_token isEqualToString:token]) return;
                            UNMutableNotificationContent *content = [[UNMutableNotificationContent alloc] init];
                            content.title = title;
                            content.body = body;
                            content.categoryIdentifier = category_id;
                            /* Baresip owns ringing. No notification sound or foreground action. */
                            UNNotificationRequest *notification = [UNNotificationRequest requestWithIdentifier:token content:content trigger:nil];
                            [center addNotificationRequest:notification withCompletionHandler:^(NSError *error) {
                                failed = error != nil;
                                /* Acceptance can arrive after our bounded wait and withdrawal.
                                 * Clean up that stale request, while preserving a live retry
                                 * using the same token for the same incoming call. */
                                dispatch_async(delivery_queue, ^{
                                    @autoreleasepool {
                                        @try {
                                            @synchronized(state_lock) {
                                                if (![active_token isEqualToString:token]) {
                                                    NSArray<NSString *> *tokens = @[token];
                                                    [center removePendingNotificationRequestsWithIdentifiers:tokens];
                                                    [center removeDeliveredNotificationsWithIdentifiers:tokens];
                                                }
                                            }
                                        } @catch (NSException *exception) {
                                            (void)exception;
                                        }
                                    }
                                });
                                dispatch_semaphore_signal(completed);
                            }];
                        }
                        /* Serialize withdrawal with acceptance up to the service deadline.
                         * The completion above cleans up acceptance arriving after a timeout. */
                        if (dispatch_semaphore_wait(completed,
                                dispatch_time(DISPATCH_TIME_NOW, SIP_NOTIFICATION_SERVICE_TIMEOUT_NS))) {
                            result = 1;
                        } else {
                            result = failed ? 1 : 0;
                        }
                    } @catch (NSException *exception) {
                        (void)exception;
                        result = 1;
                    }
                }
            });
            return result;
        } @catch (NSException *exception) {
            (void)exception;
            return 1;
        }
    }
}

int sip_call_notifications_dismiss(const char *token_bytes) {
    if (!center || NSThread.isMainThread) return 1;
    @autoreleasepool {
        @try {
            NSString *token = token_value(token_bytes);
            if (!token) return 1;
            @synchronized(state_lock) {
                if ([active_token isEqualToString:token]) active_token = nil;
            }
            __block int result = 0;
            dispatch_sync(delivery_queue, ^{
                @try {
                    NSArray<NSString *> *tokens = @[token];
                    [center removePendingNotificationRequestsWithIdentifiers:tokens];
                    [center removeDeliveredNotificationsWithIdentifiers:tokens];
                } @catch (NSException *exception) {
                    (void)exception;
                    result = 1;
                }
            });
            return result;
        } @catch (NSException *exception) {
            (void)exception;
            return 1;
        }
    }
}

int sip_test_notification_show(const char *title_bytes, const char *body_bytes) {
    if (!center || NSThread.isMainThread || !title_bytes || !body_bytes) return 1;
    @autoreleasepool {
        @try {
            int permission;
            if (read_permission(&permission) || permission != 2) return 1;
            UNMutableNotificationContent *content = [[UNMutableNotificationContent alloc] init];
            content.title = [[NSString alloc] initWithUTF8String:title_bytes];
            content.body = [[NSString alloc] initWithUTF8String:body_bytes];
            if (!content.title.length || !content.body.length) return 1;
            NSString *identifier = [test_identifier_prefix stringByAppendingString:NSUUID.UUID.UUIDString];
            UNNotificationRequest *notification = [UNNotificationRequest requestWithIdentifier:identifier content:content trigger:nil];
            dispatch_semaphore_t completed = dispatch_semaphore_create(0);
            __block BOOL failed = NO;
            /* Independent of real-call delivery_queue/active_token. ARC retains
             * the block, semaphore and byref result until even a late callback
             * finishes; no stack pointer or per-call registry escapes. */
            [center addNotificationRequest:notification withCompletionHandler:^(NSError *error) {
                failed = error != nil;
                dispatch_semaphore_signal(completed);
            }];
            if (dispatch_semaphore_wait(completed,
                    dispatch_time(DISPATCH_TIME_NOW, SIP_NOTIFICATION_SERVICE_TIMEOUT_NS))) return 1;
            return failed ? 1 : 0;
        } @catch (NSException *exception) {
            (void)exception;
            return 1;
        }
    }
}
