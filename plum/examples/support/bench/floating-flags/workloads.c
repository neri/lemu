/* Ordinary scalar algorithms, compiled unchanged for x87 and SSE2.
 * No intrinsics, flag polling in the inner loop, or artificial PE pre-setting. */
typedef unsigned int u32;
#define N 1024
static float input[N * 4];
static double prices[N];
#if CASE < 2
float output[N * 4] __attribute__((section(".output")));
#if CASE == 1
int pixels[N * 2] __attribute__((section(".output")));
#endif
#else
double result[N] __attribute__((section(".output")));
#endif

void initialize(void) {
    u32 seed = 0x12345678;
    for (u32 i = 0; i < N * 4; ++i) {
        seed = seed * 1664525u + 1013904223u;
        input[i] = ((int)(seed >> 8) - 0x800000) * (1.0f / 8388608.0f);
    }
    for (u32 i = 0; i < N; ++i)
        prices[i] = 25.0 + (double)(i % 113) * 0.37 + input[i] * 0.1;
}

u32 bench(void) {
#if CASE == 0
    /* Audio: one biquad section, gain and clipping. */
    float x1 = 0, x2 = 0, y1 = 0, y2 = 0;
    for (u32 i = 0; i < N * 4; ++i) {
        float x = input[i];
        float y = 0.067455f * x + 0.134911f * x1 + 0.067455f * x2
                + 1.14298f * y1 - 0.412802f * y2;
        x2 = x1; x1 = x; y2 = y1; y1 = y;
        float v = y * 1.1f;
        output[i] = v < -0.8f ? -0.8f : (v > 0.8f ? 0.8f : v);
    }
#elif CASE == 1
    /* Graphics: affine transform, perspective divide and viewport clipping. */
    for (u32 i = 0; i < N; ++i) {
        float x = input[4*i], y = input[4*i+1], z = input[4*i+2];
        float tx = x * 0.866025f - y * 0.5f + 0.2f;
        float ty = x * 0.5f + y * 0.866025f - 0.1f;
        float w = z * 0.15f + 2.0f;
        float sx = tx / w, sy = ty / w;
        output[4*i] = sx < -1 ? -1 : (sx > 1 ? 1 : sx);
        output[4*i+1] = sy < -1 ? -1 : (sy > 1 ? 1 : sy);
        output[4*i+2] = z;
        output[4*i+3] = w;
        pixels[2*i] = (int)(sx * 320.0f + 320.0f);
        pixels[2*i+1] = (int)(sy * 240.0f + 240.0f);
    }
#elif CASE == 2
    /* Statistics: mean/variance followed by normalization. */
    double sum = 0, squares = 0;
    for (u32 i = 0; i < N; ++i) {
        double x = prices[i];
        sum += x; squares += x*x;
    }
    double mean = sum / N;
    double variance = squares / N - mean*mean;
    double scale = variance > 0 ? 1.0 / __builtin_sqrt(variance) : 1.0;
    for (u32 i = 0; i < N; ++i)
        result[i] = (prices[i] - mean) * scale;
#elif CASE == 3
    /* Finance: weighted price update, fee and bounded risk adjustment. */
    for (u32 i = 0; i < N; ++i) {
        double p = prices[i];
        double target = prices[(i + 17) & (N - 1)];
        double change = (target - p) / p;
        double weight = change < -0.05 ? 0.05 : (change > 0.05 ? 0.95 : 0.5 + change*9.0);
        double traded = p * (1.0 - weight) + target * weight;
        result[i] = traded * (traded > 40.0 ? 0.999 : 0.9985);
    }
#endif
    return 0; /* The host verifies output bits outside the timed interval. */
}
