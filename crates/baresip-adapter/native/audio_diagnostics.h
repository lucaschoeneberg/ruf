/* Pure PCM helpers; no devices, handles, files or retained audio samples. */
#ifndef SIP_AUDIO_DIAGNOSTICS_H
#define SIP_AUDIO_DIAGNOSTICS_H
#include <stddef.h>
#include <stdint.h>
uint32_t sip_audio_peak_s16(const int16_t *samples, size_t count);
int16_t sip_ringtone_sample(int variant, unsigned volume, uint32_t index);
#endif
