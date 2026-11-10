/* Stalled Contacts consent regression; no real contact service or source records. */
#define SIP_CONTACT_AUTHORIZATION_TIMEOUT_NS (20 * NSEC_PER_MSEC)
#include "../contacts.m"
#include <stdio.h>

@interface TestContactStore : NSObject
@property(nonatomic) BOOL late;
@property(nonatomic) BOOL immediate;
@property(nonatomic, copy) dispatch_block_t pendingAuthorization;
@end
@implementation TestContactStore
- (void)requestAccessForEntityType:(CNEntityType)entity completionHandler:(void (^)(BOOL, NSError *))completion {
    (void)entity;
    if (self.immediate) completion(NO, nil);
    else if (self.late) {
        self.pendingAuthorization = ^{
            completion(NO, [NSError errorWithDomain:@"Synthetic" code:1 userInfo:nil]);
        };
    }
}
@end

int main(void) {
    @autoreleasepool {
        TestContactStore *store = [[TestContactStore alloc] init];
        NSError *error = nil;
        if (!request_authorization((CNContactStore *)(id)store, &error) || error) return 1;
        store.late = YES;
        if (!request_authorization((CNContactStore *)(id)store, &error) || error) return 2;
        store.immediate = YES;
        if (request_authorization((CNContactStore *)(id)store, &error) || error) return 3;
        dispatch_block_t completion = store.pendingAuthorization;
        store.pendingAuthorization = nil;
        if (!completion) return 4;
        completion();
        if (error) return 5;
        puts("MACOS_CONTACT_TIMEOUTS_OK: stalled authorization, late callback, denial result, subsequent request");
    }
    return 0;
}
