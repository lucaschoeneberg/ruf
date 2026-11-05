#include "audio_diagnostics.h"
#include <math.h>
#include <stdbool.h>
uint32_t sip_audio_peak_s16(const int16_t *samples, size_t count) {
    uint32_t peak = 0;
    for (size_t i = 0; i < count; ++i) {
        int32_t sample = samples[i];
        uint32_t amplitude = (uint32_t)(sample < 0 ? -sample : sample);
        if (amplitude > peak) peak = amplitude;
    }
    return peak * 1000 / 32768;
}
int16_t sip_ringtone_sample(int variant, unsigned volume, uint32_t index) {
    if (variant < 0 || variant > 2 || volume > 100) return 0;
    double seconds = (double)index / 48000;
    double local = variant == 2 ? seconds : fmod(seconds, 0.6);
    double pulse = variant == 2 ? 0.25 : 0.4;
    double edge = variant == 1 ? 0.04 : variant == 2 ? 0.02 : 0.01;
    bool on = seconds < (variant == 2 ? 0.25 : 1.0) && local < pulse;
    double fade = fmin(1.0, fmin(local / edge, (pulse - local) / edge));
    double amplitude = variant == 1 ? 1400.0 : 2200.0;
    return on ? (int16_t)(amplitude * fade * (sin(2.0 * 3.141592653589793 * 440 * seconds) +
        sin(2.0 * 3.141592653589793 * 480 * seconds)) * ((double)volume / 100)) : 0;
}
