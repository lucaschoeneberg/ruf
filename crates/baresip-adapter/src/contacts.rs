//! Copies borrowed Contacts.framework values into owned, platform-independent DTOs.

use phone_core::{
    Contact, ContactPhone, ContactSource, ContactsPermission, ContactsSnapshot, ErrorCode,
    PhoneError, contact_dial_target,
};
use std::ffi::{CStr, CString, c_char, c_int, c_void};

type ContactCallback = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
);

unsafe extern "C" {
    fn sip_contacts_revision() -> u64;
    fn sip_copy_plain_text(text: *const c_char) -> c_int;
    fn sip_contacts_permission(request_access: c_int, permission: *mut c_int) -> c_int;
    fn sip_contacts_read(
        request_access: c_int,
        permission: *mut c_int,
        callback: ContactCallback,
        context: *mut c_void,
    ) -> c_int;
}

pub(crate) fn revision() -> u64 {
    // SAFETY: native code only initializes a retained observer and reads an atomic counter.
    unsafe { sip_contacts_revision() }
}

pub(crate) fn copy_text(text: &str) -> Result<(), PhoneError> {
    let text = CString::new(text).map_err(|_| {
        PhoneError::new(
            ErrorCode::InvalidInput,
            "Clipboard text cannot contain NUL characters.",
        )
    })?;
    // SAFETY: native code synchronously copies this NUL-terminated UTF-8 value.
    if unsafe { sip_copy_plain_text(text.as_ptr()) } != 0 {
        return Err(PhoneError::new(
            ErrorCode::NativeFailure,
            "The text could not be copied to the clipboard.",
        ));
    }
    Ok(())
}

pub(crate) fn permission(request_access: bool) -> Result<ContactsPermission, PhoneError> {
    let mut permission = 0;
    // SAFETY: the synchronous native function writes only to this live stack value.
    let result = unsafe { sip_contacts_permission(i32::from(request_access), &mut permission) };
    permission_result(result, permission)
}

fn permission_result(result: c_int, permission: c_int) -> Result<ContactsPermission, PhoneError> {
    if result != 0 {
        return Err(PhoneError::new(
            ErrorCode::NativeFailure,
            "The macOS contacts permission could not be checked or requested. Try again.",
        ));
    }
    match permission {
        0 => Ok(ContactsPermission::NotDetermined),
        1 => Ok(ContactsPermission::Restricted),
        2 => Ok(ContactsPermission::Denied),
        3 => Ok(ContactsPermission::Authorized),
        _ => Err(read_error()),
    }
}

// Callback state remains on the calling worker's stack for the synchronous enumeration.
#[derive(Default)]
struct ContactRows {
    contacts: Vec<Contact>,
    invalid: bool,
}

unsafe fn string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // SAFETY: the native bridge supplies a NUL-terminated UTF-8 NSString for this callback.
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .ok()
        .map(str::to_owned)
}

unsafe extern "C" fn receive(
    context: *mut c_void,
    id: *const c_char,
    name: *const c_char,
    organization: *const c_char,
    label: *const c_char,
    number: *const c_char,
) {
    // SAFETY: read() keeps this exclusive stack-owned accumulator alive until enumeration ends.
    let rows = unsafe { &mut *context.cast::<ContactRows>() };
    // SAFETY: every pointer is borrowed only for this synchronous callback and copied here.
    let values = unsafe {
        (
            string(id),
            string(name),
            string(organization),
            string(label),
            string(number),
        )
    };
    let (Some(id), Some(name), Some(organization), Some(label), Some(number)) = values else {
        rows.invalid = true;
        return;
    };
    if id.is_empty() {
        rows.invalid = true;
        return;
    }
    if rows.contacts.last().is_none_or(|contact| contact.id != id) {
        rows.contacts.push(Contact {
            id,
            source: ContactSource::Macos,
            name,
            organization,
            phones: Vec::new(),
            favorite: false,
        });
    }
    if !number.is_empty()
        && let Some(contact) = rows.contacts.last_mut()
    {
        contact.phones.push(ContactPhone {
            destination: contact_dial_target(&number),
            label,
            number,
        });
    }
}

pub(crate) fn read(request_access: bool) -> Result<ContactsSnapshot, PhoneError> {
    let mut permission = 0;
    let mut rows = ContactRows::default();
    // SAFETY: the native function runs synchronously; its callbacks cannot outlive rows.
    let result = unsafe {
        sip_contacts_read(
            i32::from(request_access),
            &mut permission,
            receive,
            (&mut rows as *mut ContactRows).cast(),
        )
    };
    let permission = permission_result(0, permission)?;
    if result != 0 || rows.invalid {
        return Err(read_error());
    }
    if permission != ContactsPermission::Authorized {
        rows.contacts.clear();
    }
    Ok(ContactsSnapshot {
        permission,
        contacts: rows.contacts,
    })
}

fn read_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "The macOS contact store could not be read. Try refreshing the address book.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn native_permission_statuses_preserve_the_access_decision() {
        for (status, expected) in [
            (0, ContactsPermission::NotDetermined),
            (1, ContactsPermission::Restricted),
            (2, ContactsPermission::Denied),
            (3, ContactsPermission::Authorized),
        ] {
            assert!(permission_result(0, status).is_ok_and(|actual| actual == expected));
        }
    }

    #[test]
    fn native_permission_failures_never_become_an_access_decision() {
        for (result, status) in [(1, 0), (1, 2), (1, 3), (0, -1), (0, 4)] {
            let error = permission_result(result, status).err().unwrap();
            assert_eq!(error.code, ErrorCode::NativeFailure);
        }
    }

    fn row(rows: &mut ContactRows, id: &str, number: &str) {
        let fields =
            [id, "Testperson", "Testfirma", "Arbeit", number].map(|s| CString::new(s).unwrap());
        // SAFETY: these synthetic strings and the accumulator outlive the synchronous call.
        unsafe {
            receive(
                (rows as *mut ContactRows).cast(),
                fields[0].as_ptr(),
                fields[1].as_ptr(),
                fields[2].as_ptr(),
                fields[3].as_ptr(),
                fields[4].as_ptr(),
            );
        }
    }

    #[test]
    fn callback_copies_multiple_numbers_and_keeps_namesakes_separate() {
        let mut rows = ContactRows::default();
        row(&mut rows, "synthetic-a", "+49 30 123");
        row(&mut rows, "synthetic-a", "456");
        row(&mut rows, "synthetic-b", "");
        assert!(!rows.invalid);
        assert_eq!(rows.contacts.len(), 2);
        assert_eq!(rows.contacts[0].phones.len(), 2);
        assert_eq!(rows.contacts[0].phones[0].number, "+49 30 123");
        assert_eq!(
            rows.contacts[0].phones[0].destination.as_deref(),
            Some("+4930123")
        );
        assert!(rows.contacts[1].phones.is_empty());
    }
}
