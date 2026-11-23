typedef unsigned int u32;
typedef unsigned short u16;
typedef unsigned char u8;
#define INPUT ((volatile u32 *)4096)
#define BYTES ((volatile u8 *)4096)
#define WORDS ((u16 *)4096)
#define COUNTS ((u32 *)16384)
#define DEST ((volatile u8 *)24576)
__attribute__((noinline)) u32 mix(u32 x) { return (x ^ (x >> 7)) * 33 + 17; }
__attribute__((noinline)) u32 recurse(u32 n) { return n ? mix(recurse(n-1)) : 7; }
u32 bench(void) {
    u32 sum=0;
#if CASE == 0
    for (u32 i=0;i<1024;i++) {
        u32 x=INPUT[i];
        sum += x & 1 ? (x >> 3) + i : (x ^ (i*13));
        sum = (sum << 3) | (sum >> 29);
    }
#elif CASE == 1
    for (u32 i=0;i<2048;i++) {
        u32 c=BYTES[i];
        sum += c >= '0' && c <= '9' ? c-'0' : c ^ 0x20;
        sum += (int)(short)WORDS[(i+c)&2047] + (int)(signed char)BYTES[i];
    }
#elif CASE == 2 || CASE == 5 || CASE == 6
    for (u32 i=0;i<256;i++) COUNTS[i]=i;
    for (u32 i=0;i<2048;i++) {
        u32 x=BYTES[i];
#if CASE != 6
        COUNTS[x] += 3;                 /* immediate memory update */
#endif
#if CASE != 5
        COUNTS[(x+17)&255] += WORDS[(i+x)&2047];  /* register memory update */
#endif
        sum = (sum*33) ^ COUNTS[(x+31)&255];
    }
    for (u32 i=0;i<256;i++) sum += COUNTS[i];
#elif CASE == 3
    for (u32 i=0;i<1024;i++) {
        u32 x=INPUT[i];
        if ((x&15)<7) sum += mix(x);
        else sum ^= mix(i);
        if ((i&63)==0) sum += recurse(4);
    }
#elif CASE == 4
    /* Scalar and REP, short and bulk copies; consume the copied bytes. */
    for (u32 size=16;size<=2048;size*=8) {
        for (u32 i=0;i<size;i++) DEST[i]=BYTES[i];
        for (u32 i=0;i<size;i++) sum += DEST[i] ^ BYTES[(i+1)&2047];
        u32 n=size; const void *src=(const void*)4096; void *dst=(void*)24576;
        __asm__ volatile("cld; rep movsb" : "+S"(src), "+D"(dst), "+c"(n) : : "memory", "cc");
        n=size; src=(const void*)4096; dst=(void*)24576;
        __asm__ volatile("repe cmpsb" : "+S"(src), "+D"(dst), "+c"(n) : : "memory", "cc");
        sum += n;
    }
#endif
    return sum;
}
