// Measurement-only shell process. No root, cache dropping or app trim signals.
// Each newline adds 32 MiB, up to the explicit bound. EOF/q releases everything.
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>

int main(int argc, char **argv) {
    const size_t chunk = 32 * 1024 * 1024;
    char *end = NULL;
    if (argc != 2) return 2;
    long limit = strtol(argv[1], &end, 10);
    if (*end || limit < 32 || limit > 2048 || limit % 32) return 2;
    size_t count = (size_t)limit / 32;
    void **blocks = calloc(count, sizeof(*blocks));
    if (!blocks) return 3;
    // Wall-clock watchdog also releases memory if the host/controller disappears.
    alarm(180);
    setvbuf(stdout, NULL, _IOLBF, 0);
    printf("{\"pid\":%d,\"mib\":0}\n", getpid());
    uint64_t random = 0x123456789abcdefULL;
    size_t used = 0;
    char command[16];
    while (fgets(command, sizeof(command), stdin) && command[0] != 'q') {
        if (used == count) break;
        void *block = mmap(NULL, chunk, PROT_READ | PROT_WRITE,
                           MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        if (block == MAP_FAILED) return 4;
        blocks[used++] = block;
        // Fill every byte with deterministic incompressible data: merely touching
        // one zero byte/page would let zram conceal most of the pressure workload.
        volatile uint64_t *words = block;
        for (size_t i = 0; i < chunk / sizeof(*words); ++i) {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            words[i] = random;
        }
        printf("{\"pid\":%d,\"mib\":%zu}\n", getpid(), used * 32);
    }
    for (size_t i = 0; i < used; ++i) munmap(blocks[i], chunk);
    free(blocks);
    return 0;
}
