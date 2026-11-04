/* Private platform inventory; synchronous callbacks borrow strings for one call. */
#ifndef SIP_AUDIO_DEVICES_H
#define SIP_AUDIO_DEVICES_H
typedef void (sip_audio_device_h)(int input, const char *id, const char *name, int selectable, void *arg);
int sip_audio_devices(sip_audio_device_h *callback, void *arg);
int sip_audio_device_available(const char *id, int input);
#endif
