/* Stalled Contacts consent regression; no real contact service or source records. */
#define SIP_CONTACT_AUTHORIZATION_TIMEOUT_NS (20 * NSEC_PER_MSEC)
#include "../contacts.m"
#include <stdio.h>

@interface TestContactStore : NSObject
@property(nonatomic) BOOL late;
@property(nonatomic) BOOL immediate;
@property(nonatomic, strong) dispatch_semaphore_t lateCompleted;
@end
@implementation TestContactStore
- (void)requestAccessForEntityType:(CNEntityType)entity completionHandler:(void (^)(BOOL, NSError *))completion {
    (void)entity;
    if (self.immediate) completion(NO, nil);
    else if (self.late) {
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 60 * NSEC_PER_MSEC),
            dispatch_get_global_queue(QOS_CLASS_DEFAULT, 0), ^{
                completion(NO, [NSError errorWithDomain:@"Synthetic" code:1 userInfo:nil]);
                dispatch_semaphore_signal(self.lateCompleted);
            });
    }
}
@end

int main(void) {
    @autoreleasepool {
        TestContactStore *store = [[TestContactStore alloc] init];
        store.lateCompleted = dispatch_semaphore_create(0);
        NSError *error = nil;
        if (!request_authorization((CNContactStore *)(id)store, &error) || error) return 1;
        store.late = YES;
        if (!request_authorization((CNContactStore *)(id)store, &error) || error) return 2;
        store.immediate = YES;
        if (request_authorization((CNContactStore *)(id)store, &error) || error) return 3;
        if (dispatch_semaphore_wait(store.lateCompleted, dispatch_time(DISPATCH_TIME_NOW, NSEC_PER_SEC))) return 4;
        puts("MACOS_CONTACT_TIMEOUTS_OK: stalled authorization, late callback, denial result, subsequent request");
    }
    return 0;
}
