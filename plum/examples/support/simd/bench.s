# SSE benchmark guests for plum/examples/simd_bench.rs. EAX selects the
# workload (0 integer XMM, 1 floating point, 2 MMX); [0x3000] is the outer
# loop count. The compiled bytes are stored in bench.bin.hex.
# See README.md for rebuilding into target/guest-fixtures and encoding as text.
.intel_syntax noprefix
.code32
    mov ebx, [0x3000]
    cmp eax, 1
    je fp
    cmp eax, 2
    je mmx
int:                            # 12 SIMD instructions per block
    pxor xmm7, xmm7
1:  mov esi, 0x10000
    mov edi, 0x14000
    mov ecx, 256
2:  movdqa xmm0, [esi]
    movdqa xmm1, xmm0
    paddb xmm0, [0x3010]
    pxor xmm0, [0x3020]
    pcmpeqb xmm1, [0x3030]
    pand xmm1, xmm0
    psadbw xmm1, [0x3020]
    paddq xmm7, xmm1
    punpcklbw xmm0, xmm0
    psrlw xmm0, 3
    packuswb xmm0, xmm0
    movdqa [edi], xmm0
    add esi, 16
    add edi, 16
    dec ecx
    jnz 2b
    dec ebx
    jnz 1b
    jmp done
fp:                             # 10 SIMD instructions per block
    movaps xmm6, [0x3040]
    xorps xmm7, xmm7
1:  mov esi, 0x10000
    mov ecx, 256
2:  movaps xmm0, [esi]
    andps xmm0, [0x3050]
    addps xmm0, xmm6
    mulps xmm0, xmm6
    sqrtps xmm1, xmm0
    divps xmm0, xmm1
    maxps xmm7, xmm0
    cvttps2dq xmm2, xmm0
    cvtdq2ps xmm2, xmm2
    addss xmm7, xmm2
    add esi, 16
    dec ecx
    jnz 2b
    dec ebx
    jnz 1b
    jmp done
mmx:                            # 10 SIMD instructions per block
    pxor mm7, mm7
1:  mov esi, 0x10000
    mov ecx, 512
2:  movq mm0, [esi]
    movq mm1, mm0
    paddusb mm0, [0x3010]
    pmullw mm1, [0x3020]
    pmaddwd mm0, mm1
    psrad mm0, 2
    packssdw mm0, mm0
    pavgb mm1, mm0
    pxor mm7, mm1
    movq [esi + 0x4000], mm1
    add esi, 8
    dec ecx
    jnz 2b
    dec ebx
    jnz 1b
    emms
done:
