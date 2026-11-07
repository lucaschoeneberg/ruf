/* Sandboxed main-app login registration; no LaunchAgent files or helper bundle. */
#import <AppKit/AppKit.h>
#import <ServiceManagement/ServiceManagement.h>
#include "login_item.h"

static int login_item_enabled(SMAppService *service, int *enabled) {
    if (!service || !enabled) return 1;
    @try {
        switch (service.status) {
            case SMAppServiceStatusEnabled: *enabled = 1; return 0;
            case SMAppServiceStatusNotRegistered:
            case SMAppServiceStatusRequiresApproval:
            case SMAppServiceStatusNotFound: *enabled = 0; return 0;
            default: return 1;
        }
    } @catch (NSException *exception) {
        (void)exception;
        return 1;
    }
}

static int login_item_set_enabled(SMAppService *service, int enabled) {
    if (!service || (enabled != 0 && enabled != 1)) return 1;
    @try {
        SMAppServiceStatus status = service.status;
        if ((enabled && status == SMAppServiceStatusEnabled) ||
            (!enabled && (status == SMAppServiceStatusNotRegistered ||
                          status == SMAppServiceStatusNotFound))) return 0;
        /* Respect a user's OS denial; do not re-register or open Settings. */
        if (enabled && status == SMAppServiceStatusRequiresApproval) return 1;
        if (status != SMAppServiceStatusNotRegistered &&
            status != SMAppServiceStatusEnabled &&
            status != SMAppServiceStatusRequiresApproval &&
            status != SMAppServiceStatusNotFound) return 1;
        /* A fresh app can report NotFound before its first registration.
         * Let ServiceManagement validate registration instead of blocking it. */
        BOOL accepted = enabled ? [service registerAndReturnError:nil]
                                : [service unregisterAndReturnError:nil];
        if (!accepted) return 1;
        /* Command acceptance alone is not evidence of activation/removal. */
        return service.status == (enabled ? SMAppServiceStatusEnabled
                                         : SMAppServiceStatusNotRegistered) ? 0 : 1;
    } @catch (NSException *exception) {
        (void)exception;
        return 1;
    }
}

static BOOL login_item_launch_event(NSAppleEventDescriptor *event) {
    return event && event.eventClass == kCoreEventClass &&
        event.eventID == kAEOpenApplication &&
        [event paramDescriptorForKeyword:keyAEPropData].enumCodeValue == keyAELaunchedAsLogInItem;
}

int sip_login_item_enabled(int *enabled) {
    if (!enabled) return 1;
    @autoreleasepool {
        @try {
            return login_item_enabled(SMAppService.mainAppService, enabled);
        } @catch (NSException *exception) {
            (void)exception;
            return 1;
        }
    }
}

int sip_login_item_set_enabled(int enabled) {
    @autoreleasepool {
        @try {
            return login_item_set_enabled(SMAppService.mainAppService, enabled);
        } @catch (NSException *exception) {
            (void)exception;
            return 1;
        }
    }
}

int sip_login_item_launched_at_login(void) {
    /* Startup setup runs on the main thread. Subsequent single-instance
     * activation may run on a worker and must not reuse the initial event. */
    if (![NSThread isMainThread]) return 0;
    @autoreleasepool {
        @try {
            return login_item_launch_event(NSAppleEventManager.sharedAppleEventManager.currentAppleEvent) ? 1 : 0;
        } @catch (NSException *exception) {
            (void)exception;
            return 0;
        }
    }
}
