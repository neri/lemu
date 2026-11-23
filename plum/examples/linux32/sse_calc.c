/* Freestanding Linux/i386 app compiled for SSE2 (no libc): the same ELF runs
 * natively and in UME, and the outputs must match (tests/linux32.rs).
 * gcc -O3 -msse2 -mfpmath=sse vectorizes the loops below (MOVDQU, PADDB,
 * PMAXSW, PMINUB, ADDPS, MULPS, CMPLTPS, ...) and uses scalar SSE2 for
 * double arithmetic, SQRTSD and the conversions. Every result is printed as
 * hexadecimal bits, so the comparison is exact. */
typedef unsigned int u32;
typedef unsigned long long u64;
typedef unsigned char u8;

static int syscall3(u32 number, u32 a, u32 b, u32 c) {
    int result;
    __asm__ volatile("int $0x80" : "=a"(result)
                     : "0"(number), "b"(a), "c"(b), "d"(c) : "memory", "cc");
    return result;
}

static char out[4096];
static u32 used;

static void text(const char *s) {
    while (*s) out[used++] = *s++;
}

static void line(const char *name, u64 value) {
    text(name);
    out[used++] = ' ';
    for (int shift = 60; shift >= 0; shift -= 4)
        out[used++] = "0123456789abcdef"[(value >> shift) & 15];
    out[used++] = '\n';
}

static u64 bits_d(double d) {
    union { double d; u64 u; } x = { .d = d };
    return x.u;
}

static u32 bits_f(float f) {
    union { float f; u32 u; } x = { .f = f };
    return x.u;
}

static void set_csr(u32 value) { __asm__ volatile("ldmxcsr %0" : : "m"(value)); }
static u32 get_csr(void) { u32 value; __asm__ volatile("stmxcsr %0" : "=m"(value)); return value; }

static float fa[256], fb[256], fc[256];
static u8 bytes[1024], bytes2[1024];
static short words[512];
static int ints[256];

__attribute__((force_align_arg_pointer, noreturn)) void _start(void) {
    u32 a, b, c, d;
    __asm__ volatile("cpuid" : "=a"(a), "=b"(b), "=c"(c), "=d"(d) : "0"(1));
    line("cpuid.sse2", (d >> 26) & 1);

    for (int i = 0; i < 256; i++) {
        fa[i] = (float)(i * 3 + 1) / 7.0f;
        fb[i] = 1.0f / (float)(i + 1);
        ints[i] = (int)((u32)i * 2654435761u);
    }
    for (int i = 0; i < 1024; i++) bytes[i] = (u8)((i * 37) ^ (i >> 3));
    for (int i = 0; i < 512; i++) words[i] = (short)(i * 977 - 30000);

    /* Packed single: multiply-add and a compare count. */
    for (int i = 0; i < 256; i++) fc[i] = fa[i] * fb[i] + fa[i];
    u64 h = 0;
    for (int i = 0; i < 256; i++) h = h * 31 + bits_f(fc[i]);
    line("float.madd", h);
    u32 below = 0;
    for (int i = 0; i < 256; i++) below += fa[i] < fb[i];
    line("float.below", below);

    /* Scalar double: a sum, sqrt, division, conversions. */
    double s = 0;
    for (int i = 1; i <= 2000; i++) s += 1.0 / ((double)i * (double)i);
    line("double.basel", bits_d(s));
    line("double.sqrt", bits_d(__builtin_sqrt(s * 6.0)));
    int truncated = 0;
    for (int i = 0; i < 256; i++) truncated += (int)(fa[i] * 100.0f) - (int)(fb[i] * -1000.0);
    line("convert.truncate", (u32)truncated);
    line("convert.narrow", bits_f((float)(s / 3.0)));
    line("convert.widen", bits_d((double)fb[100] * 3.0));

    /* Integer vectors. */
    u32 sum = 0;
    for (int i = 0; i < 1024; i++) sum += bytes[i];
    line("int.byte_sum", sum);
    short wmax = -32768;
    for (int i = 0; i < 512; i++) if (words[i] > wmax) wmax = words[i];
    line("int.word_max", (u32)(unsigned short)wmax);
    u8 bmin = 255;
    for (int i = 0; i < 1024; i++) if (bytes[i] < bmin + 0 && bytes[i] > 3) bmin = bytes[i];
    line("int.byte_min", bmin);
    for (int i = 0; i < 1024; i++) bytes2[i] = bytes[i] > 200 ? 255 : (u8)(bytes[i] + 55);
    u64 bh = 0;
    for (int i = 0; i < 1024; i++) bh = bh * 131 + bytes2[i];
    line("int.saturate", bh);
    u32 x = 0;
    for (int i = 0; i < 256; i++) x ^= (u32)ints[i] * 3u + ((u32)ints[i] >> 5);
    line("int.xor", x);
    int greater = 0;
    for (int i = 0; i < 256; i++) greater += ints[i] > ints[255 - i];
    line("int.greater", (u32)greater);

    /* Rounding modes (every exception masked). */
    volatile double one = 1.0, three = 3.0, half = 2.5;
    for (u32 rc = 0; rc < 4; rc++) {
        char name[] = "round.0";
        name[6] = (char)('0' + rc);
        set_csr(0x1f80 | rc << 13);
        double third = one / three;
        int rounded;
        __asm__ volatile("cvtsd2si %1, %0" : "=r"(rounded) : "x"(half));
        line(name, bits_d(third));
        line(name, (u32)rounded);
    }
    set_csr(0x1f80);

    /* NaN, the default NaN and the integer indefinite. */
    volatile double zero = 0.0, big = 3e9;
    line("nan.zero_over_zero", bits_d(zero / zero));
    line("nan.sqrt_negative", bits_d(__builtin_sqrt(-one)));
    line("nan.inf_minus_inf", bits_d(1.0 / zero - 1.0 / zero));
    line("indefinite", (u32)(int)big);

    line("mxcsr", get_csr());

    syscall3(4, 1, (u32)out, used);
    syscall3(1, 0, 0, 0);
    __builtin_unreachable();
}
