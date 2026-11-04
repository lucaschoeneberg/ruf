/* Device metadata only. Opening settings never opens an audio input stream. */
#ifdef WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#define CINTERFACE
#define COBJMACROS
#include <initguid.h>
#include <windows.h>
#include <mmdeviceapi.h>
#include <functiondiscoverykeys_devpkey.h>
#endif
#include <string.h>
#include <stdlib.h>
#include <re.h>
#include <baresip.h>
#include "audio_devices.h"

#ifdef __APPLE__
#include <CoreFoundation/CoreFoundation.h>
#include <CoreAudio/CoreAudio.h>
/* Public function of the pinned upstream coreaudio module (not an OS audio pipeline). */
extern int coreaudio_enum_devices(const char *, struct list *, CFStringRef *, Boolean);
/* Upstream routes by display name. Reject ambiguous names rather than sending
   a call to whichever identically named USB endpoint happens to enumerate first. */
static int coreaudio_name_count(const char *name, bool input) {
    AudioObjectPropertyAddress property = {kAudioHardwarePropertyDevices, kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyElementMain};
    UInt32 size = 0;
    if (AudioObjectGetPropertyDataSize(kAudioObjectSystemObject, &property, 0, NULL, &size)) return -1;
    AudioDeviceID *ids = malloc(size);
    if (!ids) return -1;
    if (AudioObjectGetPropertyData(kAudioObjectSystemObject, &property, 0, NULL, &size, ids)) { free(ids); return -1; }
    UInt32 count = size / sizeof(*ids);
    int matches = 0;
    for (UInt32 index = 0; index < count; ++index) {
        property.mScope = input ? kAudioDevicePropertyScopeInput : kAudioDevicePropertyScopeOutput;
        property.mSelector = kAudioDevicePropertyStreams;
        UInt32 streams = 0;
        if (AudioObjectGetPropertyDataSize(ids[index], &property, 0, NULL, &streams) || !streams) continue;
        property.mSelector = kAudioDevicePropertyDeviceNameCFString;
        CFStringRef value = NULL;
        UInt32 length = sizeof(value);
        if (AudioObjectGetPropertyData(ids[index], &property, 0, NULL, &length, &value)) continue;
        char buffer[512];
        if (value && CFStringGetCString(value, buffer, sizeof(buffer), kCFStringEncodingUTF8) && !str_casecmp(buffer, name)) ++matches;
        if (value) CFRelease(value);
    }
    free(ids);
    return matches;
}
#endif

int sip_audio_devices(sip_audio_device_h *callback, void *arg) {
    if (!callback) return EINVAL;
#ifdef __APPLE__
    struct list inputs = LIST_INIT, outputs = LIST_INIT;
    int error = coreaudio_enum_devices(NULL, &inputs, NULL, true);
    if (!error) error = coreaudio_enum_devices(NULL, &outputs, NULL, false);
    if (!error) {
        for (int input = 0; input <= 1; ++input) {
            struct le *le;
            LIST_FOREACH(input ? &inputs : &outputs, le) {
                const struct mediadev *device = le->data;
                int matches = coreaudio_name_count(device->name, input != 0);
                if (matches < 0) { error = ENODEV; break; }
                if (strlen(device->name) <= 127) callback(input, device->name, device->name, matches == 1, arg);
            }
        }
    }
    list_flush(&inputs); list_flush(&outputs);
    return error;
#elif defined(WIN32)
    /* The same Windows endpoint identifiers accepted by baresip's WASAPI module. */
    static const CLSID sip_enumerator_class = {0xbcde0395,0xe52f,0x467c,{0x8e,0x3d,0xc4,0x57,0x92,0x91,0x69,0x2e}};
    static const IID sip_enumerator_iid = {0xa95664d2,0x9614,0x4f35,{0xa7,0x46,0xde,0x8d,0xb6,0x36,0x17,0xe6}};
    HRESULT initialized = CoInitializeEx(NULL, COINIT_MULTITHREADED);
    if (FAILED(initialized) && initialized != RPC_E_CHANGED_MODE) return ENODEV;
    IMMDeviceEnumerator *enumerator = NULL;
    HRESULT hr = CoCreateInstance(&sip_enumerator_class, NULL, CLSCTX_ALL, &sip_enumerator_iid, (void **)&enumerator);
    int error = FAILED(hr) ? ENODEV : 0;
    for (int input = 0; !error && input <= 1; ++input) {
        IMMDeviceCollection *devices = NULL;
        UINT count = 0;
        hr = IMMDeviceEnumerator_EnumAudioEndpoints(enumerator, input ? eCapture : eRender, DEVICE_STATE_ACTIVE, &devices);
        if (SUCCEEDED(hr)) hr = IMMDeviceCollection_GetCount(devices, &count);
        if (FAILED(hr)) error = ENODEV;
        for (UINT index = 0; !error && index < count; ++index) {
            IMMDevice *device = NULL;
            LPWSTR id = NULL;
            IPropertyStore *properties = NULL;
            PROPVARIANT name;
            char id_utf8[128], name_utf8[512];
            PropVariantInit(&name);
            hr = IMMDeviceCollection_Item(devices, index, &device);
            if (SUCCEEDED(hr)) hr = IMMDevice_GetId(device, &id);
            if (SUCCEEDED(hr)) hr = IMMDevice_OpenPropertyStore(device, STGM_READ, &properties);
            if (SUCCEEDED(hr)) hr = IPropertyStore_GetValue(properties, &PKEY_Device_FriendlyName, &name);
            if (SUCCEEDED(hr) && name.vt == VT_LPWSTR &&
                WideCharToMultiByte(CP_UTF8, 0, id, -1, id_utf8, sizeof(id_utf8), NULL, NULL) &&
                WideCharToMultiByte(CP_UTF8, 0, name.pwszVal, -1, name_utf8, sizeof(name_utf8), NULL, NULL)) callback(input, id_utf8, name_utf8, 1, arg);
            if (FAILED(hr)) error = ENODEV;
            PropVariantClear(&name);
            if (properties) IPropertyStore_Release(properties);
            if (id) CoTaskMemFree(id);
            if (device) IMMDevice_Release(device);
        }
        if (devices) IMMDeviceCollection_Release(devices);
    }
    if (enumerator) IMMDeviceEnumerator_Release(enumerator);
    if (SUCCEEDED(initialized)) CoUninitialize();
    return error;
#else
    (void)arg;
    return ENOTSUP;
#endif
}

struct device_match { const char *id; int input; bool found; bool ambiguous; };
static void find_device(int input, const char *id, const char *name, int selectable, void *arg) {
    struct device_match *match = arg;
    (void)name;
    if (input == match->input && !strcmp(id, match->id)) { match->found = true; match->ambiguous = !selectable; }
}
int sip_audio_device_available(const char *id, int input) {
    if (!id || !*id || !strcmp(id, "default")) return 0;
    struct device_match match = {id, input, false, false};
    int error = sip_audio_devices(find_device, &match);
    return error ? error : match.ambiguous ? EADDRNOTAVAIL : match.found ? 0 : ENODEV;
}
