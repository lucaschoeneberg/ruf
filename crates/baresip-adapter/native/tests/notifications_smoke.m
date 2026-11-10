/* Isolated macOS delivery/category/withdrawal check. No SIP, keychain or address book. */
#import <AppKit/AppKit.h>
#import <UserNotifications/UserNotifications.h>
#include "call_notifications.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int observed_action;
static char observed_token[129];
static void received(int action, const char *token) {
    observed_action = action;
    snprintf(observed_token, sizeof(observed_token), "%s", token);
}

/* OS-delivered notifications supply the real request. Only the user response is
 * synthetic, so this does not claim an interactive Notification Center test. */
@interface TestResponse : NSObject
@property(nonatomic, strong) UNNotification *notification;
@property(nonatomic, copy) NSString *actionIdentifier;
@end
@implementation TestResponse
@end

static void check_category(void) {
    dispatch_semaphore_t done = dispatch_semaphore_create(0);
    [UNUserNotificationCenter.currentNotificationCenter getNotificationCategoriesWithCompletionHandler:^(NSSet<UNNotificationCategory *> *categories) {
        BOOL valid = NO;
        for (UNNotificationCategory *category in categories) {
            if (category.actions.count != 2) continue;
            UNNotificationAction *accept = category.actions[0], *reject = category.actions[1];
            valid = [accept.identifier isEqualToString:@"accept"] && [accept.title isEqualToString:@"Annehmen"] &&
                    [reject.identifier isEqualToString:@"reject"] && [reject.title isEqualToString:@"Ablehnen"] &&
                    !accept.options && !reject.options;
        }
        if (!valid) exit(3);
        dispatch_semaphore_signal(done);
    }];
    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
}

static void check_delivery(int action) {
    NSString *token = [NSString stringWithFormat:@"%016d", action];
    if (sip_call_notifications_show(token.UTF8String, "Ada Beispiel", "+4930000000 · Testkonto", "Annehmen", "Ablehnen")) exit(4);
    check_category();
    [NSThread sleepForTimeInterval:1];
    dispatch_semaphore_t done = dispatch_semaphore_create(0);
    [UNUserNotificationCenter.currentNotificationCenter getDeliveredNotificationsWithCompletionHandler:^(NSArray<UNNotification *> *notices) {
        UNNotification *found = nil;
        for (UNNotification *notice in notices) {
            if ([notice.request.identifier isEqualToString:token]) found = notice;
        }
        if (!found || ![found.request.content.title isEqualToString:@"Ada Beispiel"] ||
            ![found.request.content.body containsString:@"+4930000000"] || found.request.content.sound) exit(5);
        TestResponse *response = [[TestResponse alloc] init];
        response.notification = found;
        response.actionIdentifier = action == 1 ? @"accept" : action == 2 ? @"reject" : UNNotificationDefaultActionIdentifier;
        __block int completions = 0;
        [UNUserNotificationCenter.currentNotificationCenter.delegate
            userNotificationCenter:UNUserNotificationCenter.currentNotificationCenter
            didReceiveNotificationResponse:(UNNotificationResponse *)(id)response
            withCompletionHandler:^{ completions++; }];
        if (observed_action != action || strcmp(observed_token, token.UTF8String) || completions != 1) exit(6);
        dispatch_semaphore_signal(done);
    }];
    dispatch_semaphore_wait(done, DISPATCH_TIME_FOREVER);
    if (sip_call_notifications_dismiss(token.UTF8String)) exit(7);
    printf("DELIVERY_ACTION_WITHDRAWAL_OK %d\n", action);
    fflush(stdout);
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
        if (sip_call_notifications_init("org.rufapp.notification-smoke", received)) return 1;
        BOOL request = argc == 2 && !strcmp(argv[1], "--request-permission");
        dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0), ^{
            int permission = 0;
            if (sip_call_notifications_permission(request, &permission) || permission != 2) {
                fprintf(stderr, "Notification access unavailable. Run with --request-permission or enable Ruf Notification Test in System Settings.\n");
                exit(2);
            }
            for (int action = 1; action <= 3; action++) check_delivery(action);
            [NSThread sleepForTimeInterval:1];
            [UNUserNotificationCenter.currentNotificationCenter getDeliveredNotificationsWithCompletionHandler:^(NSArray<UNNotification *> *notices) {
                if (notices.count) exit(8);
                puts("MACOS_CALL_NOTIFICATIONS_OK: delivery, categories, synthetic responses, withdrawal");
                fflush(stdout);
                exit(0);
            }];
        });
        [NSApp run];
    }
    return 0;
}
