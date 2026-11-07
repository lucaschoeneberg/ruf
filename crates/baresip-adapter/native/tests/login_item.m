/* Synthetic ServiceManagement regression; never registers a real login item.
 * clang -fobjc-arc -fblocks -Wall -Wextra -Werror -mmacosx-version-min=14.0
 *   login_item.m -framework AppKit -framework Foundation -framework ServiceManagement
 */
#include "../login_item.m"
#include <assert.h>
#include <stdio.h>

@interface TestLoginItem : NSObject
@property(nonatomic) SMAppServiceStatus status;
@property(nonatomic) BOOL fail;
@property(nonatomic) BOOL throws;
@property(nonatomic) BOOL approvalAfterRegister;
@property(nonatomic) BOOL staleAfterUnregister;
@property(nonatomic) int registrations;
@property(nonatomic) int removals;
@end
@implementation TestLoginItem
- (BOOL)registerAndReturnError:(NSError **)error {
    (void)error;
    self.registrations++;
    if (self.throws) [NSException raise:@"Synthetic" format:@"private-native-detail"];
    if (self.fail) return NO;
    self.status = self.approvalAfterRegister ? SMAppServiceStatusRequiresApproval : SMAppServiceStatusEnabled;
    return YES;
}
- (BOOL)unregisterAndReturnError:(NSError **)error {
    (void)error;
    self.removals++;
    if (self.throws) [NSException raise:@"Synthetic" format:@"private-native-detail"];
    if (self.fail) return NO;
    if (!self.staleAfterUnregister) self.status = SMAppServiceStatusNotRegistered;
    return YES;
}
@end

int main(void) {
    @autoreleasepool {
        TestLoginItem *item = [TestLoginItem new];
        SMAppService *service = (SMAppService *)(id)item;
        int enabled = -1;
        assert(sip_login_item_enabled(NULL) != 0);
        assert(login_item_enabled(nil, &enabled) != 0);
        for (int status = 0; status <= 3; status++) {
            item.status = status;
            assert(login_item_enabled(service, &enabled) == 0);
            assert(enabled == (status == SMAppServiceStatusEnabled));
        }
        item.status = SMAppServiceStatusNotFound;
        assert(login_item_set_enabled(service, 0) == 0);
        assert(login_item_set_enabled(service, 1) == 0);
        assert(item.registrations == 1);
        item.registrations = 0;
        item.status = 4;
        assert(login_item_enabled(service, &enabled) != 0);
        assert(login_item_set_enabled(service, 1) != 0);
        assert(login_item_set_enabled(service, 2) != 0);
        item.status = SMAppServiceStatusRequiresApproval;
        assert(login_item_set_enabled(service, 1) != 0);
        assert(item.registrations == 0);
        assert(login_item_set_enabled(service, 0) == 0);
        assert(item.status == SMAppServiceStatusNotRegistered);
        assert(login_item_set_enabled(service, 1) == 0);
        assert(item.registrations == 1);
        assert(login_item_set_enabled(service, 1) == 0);
        assert(item.registrations == 1);
        item.staleAfterUnregister = YES;
        assert(login_item_set_enabled(service, 0) != 0);
        item.staleAfterUnregister = NO;
        assert(login_item_set_enabled(service, 0) == 0);
        assert(login_item_set_enabled(service, 0) == 0);
        item.approvalAfterRegister = YES;
        assert(login_item_set_enabled(service, 1) != 0);
        assert(login_item_enabled(service, &enabled) == 0 && !enabled);
        item.status = SMAppServiceStatusNotRegistered;
        item.fail = YES;
        assert(login_item_set_enabled(service, 1) != 0);
        item.fail = NO;
        item.throws = YES;
        assert(login_item_set_enabled(service, 1) != 0);

        NSAppleEventDescriptor *event = [NSAppleEventDescriptor appleEventWithEventClass:kCoreEventClass
            eventID:kAEOpenApplication targetDescriptor:nil returnID:kAutoGenerateReturnID transactionID:kAnyTransactionID];
        assert(!login_item_launch_event(nil));
        assert(!login_item_launch_event(event));
        [event setParamDescriptor:[NSAppleEventDescriptor descriptorWithEnumCode:keyAELaunchedAsLogInItem] forKeyword:keyAEPropData];
        assert(login_item_launch_event(event));
        [event setParamDescriptor:[NSAppleEventDescriptor descriptorWithEnumCode:keyAELaunchedAsServiceItem] forKeyword:keyAEPropData];
        assert(!login_item_launch_event(event));
        puts("MACOS_LOGIN_ITEM_OK: real-enabled status only, approval, errors, idempotency, launch-event policy; no real mutations");
    }
    return 0;
}
