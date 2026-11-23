/* Freestanding Linux/i386 app: no libc, run the same ELF natively or in UME. */
typedef unsigned int u32;
typedef unsigned char u8;

static char input[96]; /* ELF .bss must initially be zero. */

static int syscall3(u32 number, u32 a, u32 b, u32 c) {
    int result;
    __asm__ volatile("int $0x80" : "=a"(result)
                     : "0"(number), "b"(a), "c"(b), "d"(c) : "memory", "cc");
    return result;
}

static u8 *allocate_code(void) {
    int result;
    __asm__ volatile("push %%ebp; xor %%ebp,%%ebp; int $0x80; pop %%ebp"
                     : "=a"(result)
                     : "0"(192), "b"(0), "c"(4096), "d"(7),
                       "S"(0x22), "D"(-1) : "memory", "cc");
    return (u8 *)result;
}

static void print(const char *text, u32 value) {
    char buffer[64];
    u32 length = 0, begin;
    while (*text) buffer[length++] = *text++;
    begin = length;
    do {
        buffer[length++] = '0' + value % 10;
        value /= 10;
    } while (value);
    for (u32 i = begin, j = length - 1; i < j; i++, j--) {
        char c = buffer[i]; buffer[i] = buffer[j]; buffer[j] = c;
    }
    buffer[length++] = '\n';
    if (syscall3(4, 1, (u32)buffer, length) != (int)length)
        syscall3(1, 3, 0, 0);
}

static void immediate(volatile u8 *p, u32 value) {
    for (u32 i = 0; i < 4; i++) p[i] = value >> (i * 8);
}

int app(const u32 *stack) {
    if (input[0] || stack[0] != 1 || !stack[1] || stack[2]) return 4;
    /* Three nonnegative decimal integers, bounded to six digits each. */
    int count = 0;
    while (count < (int)sizeof(input)) {
        int n = syscall3(3, 0, (u32)(input + count), 1);
        if (n != 1) break;
        if (input[count++] == '\n') break;
    }
    if (count <= 0) return 2;
    u32 values[3] = {0, 0, 0}, pos = 0;
    for (u32 i = 0; i < 3; i++) {
        u32 digits = 0;
        while (pos < (u32)count && (input[pos] == ' ' || input[pos] == '\n')) pos++;
        while (pos < (u32)count && input[pos] >= '0' && input[pos] <= '9') {
            if (++digits > 6) return 2;
            values[i] = values[i] * 10 + input[pos++] - '0';
        }
        if (!digits) return 2;
    }
    volatile u8 *code = allocate_code();
    if ((u32)code >= 0xfffff001u) return 5;
    /* MOV EAX,[ESP+4]; IMUL EAX,EAX,a; ADD EAX,b; RET */
    static const u8 template[] = {
        0x8b, 0x44, 0x24, 4, 0x69, 0xc0, 0, 0, 0, 0, 0x05, 0, 0, 0, 0, 0xc3
    };
    for (u32 i = 0; i < sizeof(template); i++) code[i] = template[i];
    immediate(code + 6, values[0]);
    immediate(code + 11, values[1]);
    u32 (*calculate)(u32) = (void *)code;
    u32 result = 0;
    for (u32 i = 0; i < 64; i++) result = calculate(values[2]);
    if (result != values[0] * values[2] + values[1]) return 6;
    print("initial: ", result);
    immediate(code + 6, values[0] + 1);
    immediate(code + 11, values[1] + 2);
    for (u32 i = 0; i < 64; i++) result = calculate(values[2]);
    if (result != (values[0] + 1) * values[2] + values[1] + 2) return 7;
    print("patched: ", result);
    /* Replace the cached 16-byte function with MOV EAX,99; RET (6 bytes). */
    code[0] = 0xb8;
    immediate(code + 1, 99);
    code[5] = 0xc3;
    result = calculate(values[2]);
    if (result != 99) return 8;
    print("replaced: ", result);
    /* Optional six-byte replacement read directly into already cached code. */
    int replacement = syscall3(3, 0, (u32)code, 6);
    if (replacement == 6) print("host patched: ", calculate(values[2]));
    else if (replacement != 0) return 9;
    return 0;
}

__asm__(".global _start\n_start:\nmov %esp,%eax\npush %eax\ncall app\n"
        "mov %eax,%ebx\nmov $1,%eax\nint $0x80\n");
