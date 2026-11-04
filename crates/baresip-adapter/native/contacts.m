#import <AppKit/AppKit.h>
#import <Contacts/Contacts.h>
#import <Foundation/Foundation.h>
#import <dispatch/dispatch.h>
#include "contacts.h"
#include <string.h>
#include <stdatomic.h>

/* Embedded NULs must never turn a source number into a different dial target. */
static const char *utf8(NSString *value) {
    if (!value) return "";
    const char *bytes = value.UTF8String;
    return bytes && strlen(bytes) == [value lengthOfBytesUsingEncoding:NSUTF8StringEncoding]
        ? bytes : NULL;
}

static int permission_value(CNAuthorizationStatus status) {
    switch (status) {
        case CNAuthorizationStatusNotDetermined: return 0;
        case CNAuthorizationStatusRestricted: return 1;
        case CNAuthorizationStatusDenied: return 2;
        case CNAuthorizationStatusAuthorized: return 3;
        default: return 1;
    }
}

static int authorize(CNContactStore *store, int request_access, int *permission) {
    CNAuthorizationStatus status = [CNContactStore authorizationStatusForEntityType:CNEntityTypeContacts];
    if (request_access && status == CNAuthorizationStatusNotDetermined) {
        /* Called only from a Rust blocking worker, never the UI thread.
         * The completion queue is independent of this waiting worker. */
        dispatch_semaphore_t completed = dispatch_semaphore_create(0);
        __block NSError *request_error = nil;
        [store requestAccessForEntityType:CNEntityTypeContacts completionHandler:^(BOOL granted, NSError *error) {
            (void)granted;
            request_error = error;
            dispatch_semaphore_signal(completed);
        }];
        dispatch_semaphore_wait(completed, DISPATCH_TIME_FOREVER);
        status = [CNContactStore authorizationStatusForEntityType:CNEntityTypeContacts];
        *permission = permission_value(status);
        /* An ordinary denial is an access decision; infrastructure errors are failures. */
        if (request_error && !(status == CNAuthorizationStatusDenied &&
            [request_error.domain isEqualToString:CNErrorDomain] &&
            request_error.code == CNErrorCodeAuthorizationDenied)) return 1;
    }
    *permission = permission_value(status);
    return 0;
}

int sip_contacts_permission(int request_access, int *permission) {
    if (!permission) return 1;
    @autoreleasepool {
        @try {
            CNContactStore *store = [[CNContactStore alloc] init];
            return authorize(store, request_access, permission);
        } @catch (NSException *exception) {
            (void)exception;
            return 1;
        }
    }
}

int sip_contacts_read(int request_access, int *permission,
    sip_contact_callback callback, void *context) {
    if (!permission || !callback || !context) return 1;
    @autoreleasepool {
        @try {
            CNContactStore *store = [[CNContactStore alloc] init];
            if (authorize(store, request_access, permission)) return 1;
            if (*permission < 3) return 0;

            NSArray<id<CNKeyDescriptor>> *keys = @[
                CNContactIdentifierKey,
                [CNContactFormatter descriptorForRequiredKeysForStyle:CNContactFormatterStyleFullName],
                CNContactOrganizationNameKey,
                CNContactPhoneNumbersKey
            ];
            CNContactFetchRequest *request = [[CNContactFetchRequest alloc] initWithKeysToFetch:keys];
            request.sortOrder = CNContactSortOrderUserDefault;
            request.unifyResults = YES;
            NSError *error = nil;
            BOOL success = [store enumerateContactsWithFetchRequest:request error:&error usingBlock:^(CNContact *contact, BOOL *stop) {
                (void)stop;
                @autoreleasepool {
                    NSString *name = [CNContactFormatter stringFromContact:contact style:CNContactFormatterStyleFullName];
                    if (!name.length) name = contact.organizationName;
                    if (!name.length) name = contact.phoneNumbers.firstObject.value.stringValue;
                    if (!name.length) name = @"Ohne Namen";
                    const char *identifier = utf8(contact.identifier);
                    const char *organization = utf8(contact.organizationName);
                    if (!contact.phoneNumbers.count) {
                        callback(context, identifier, utf8(name), organization, "", "");
                    }
                    for (CNLabeledValue<CNPhoneNumber *> *phone in contact.phoneNumbers) {
                        NSString *label = phone.label.length
                            ? [CNLabeledValue localizedStringForLabel:phone.label] : @"Telefon";
                        callback(context, identifier, utf8(name), organization,
                            utf8(label), utf8(phone.value.stringValue));
                    }
                }
            }];
            /* Permission can change while enumerating: discard all partial rows in Rust. */
            *permission = permission_value([CNContactStore authorizationStatusForEntityType:CNEntityTypeContacts]);
            if (*permission < 3) return 0;
            return success && !error ? 0 : 1;
        } @catch (NSException *exception) {
            (void)exception; /* Never print personal contact data or native exception details. */
            return 1;
        }
    }
}


/* Only a revision crosses the native boundary, never notification/contact data.
 * Keep a store alive so Contacts.framework continues issuing change events. */
static CNContactStore *revision_store;
static id revision_observer;
static atomic_uint_fast64_t contact_revision;

uint64_t sip_contacts_revision(void) {
    static dispatch_once_t once;
    dispatch_once(&once, ^{
        @autoreleasepool {
            @try {
                revision_store = [[CNContactStore alloc] init];
                revision_observer = [[NSNotificationCenter defaultCenter]
                    addObserverForName:CNContactStoreDidChangeNotification
                    object:nil queue:nil usingBlock:^(NSNotification *notification) {
                        (void)notification;
                        atomic_fetch_add_explicit(&contact_revision, 1, memory_order_relaxed);
                    }];
                if (revision_store && revision_observer)
                    atomic_fetch_add_explicit(&contact_revision, 1, memory_order_relaxed);
            } @catch (NSException *exception) {
                (void)exception;
            }
        }
    });
    return atomic_load_explicit(&contact_revision, memory_order_relaxed);
}

int sip_copy_phone_number(const char *number) {
    if (!number) return 1;
    @autoreleasepool {
        @try {
            size_t length = strlen(number);
            if (!length || length > 1024) return 1;
            NSString *text = [[NSString alloc] initWithBytes:number length:length encoding:NSUTF8StringEncoding];
            if (!text || [text rangeOfCharacterFromSet:NSCharacterSet.controlCharacterSet].location != NSNotFound) return 1;
            __block int result = 1;
            void (^copy)(void) = ^{
                @try {
                    NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
                    [pasteboard clearContents];
                    result = [pasteboard setString:text forType:NSPasteboardTypeString] ? 0 : 1;
                } @catch (NSException *exception) {
                    (void)exception;
                }
            };
            if (NSThread.isMainThread) copy();
            else dispatch_sync(dispatch_get_main_queue(), copy);
            return result;
        } @catch (NSException *exception) {
            (void)exception;
            return 1;
        }
    }
}
