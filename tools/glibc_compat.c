#define _GNU_SOURCE
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <stddef.h>
#include <sys/types.h>

// 1. Force libm math functions to bind to GLIBC_2.4 instead of GLIBC_2.27
__asm__(".symver expf, expf@GLIBC_2.4");
__asm__(".symver logf, logf@GLIBC_2.4");
__asm__(".symver powf, powf@GLIBC_2.4");
__asm__(".symver exp, exp@GLIBC_2.4");
__asm__(".symver log, log@GLIBC_2.4");
__asm__(".symver pow, pow@GLIBC_2.4");
__asm__(".symver log2f, log2f@GLIBC_2.4");
__asm__(".symver log2, log2@GLIBC_2.4");
__asm__(".symver exp2f, exp2f@GLIBC_2.4");
__asm__(".symver exp2, exp2@GLIBC_2.4");

// 2. Force common libc functions with GLIBC 2.27/2.28 revisions to bind to GLIBC_2.4
__asm__(".symver glob, glob@GLIBC_2.4");
__asm__(".symver glob64, glob64@GLIBC_2.4");
__asm__(".symver fcntl64, fcntl64@GLIBC_2.4");
__asm__(".symver fstatat64, fstatat64@GLIBC_2.4");

// 3. Provide local getrandom & getentropy implementation
// Kindle OS 5.14 kernel is Linux 3.0.35, which lacks SYS_getrandom syscall.
// Reading directly from /dev/urandom ensures 100% reliable entropy on device.
ssize_t getrandom(void *buf, size_t buflen, unsigned int flags) {
    int fd = open("/dev/urandom", O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        return -1;
    }
    size_t total = 0;
    while (total < buflen) {
        ssize_t n = read(fd, (char*)buf + total, buflen - total);
        if (n <= 0) {
            if (n < 0 && errno == EINTR) continue;
            close(fd);
            return -1;
        }
        total += (size_t)n;
    }
    close(fd);
    return (ssize_t)total;
}

int getentropy(void *buffer, size_t length) {
    if (length > 256) {
        errno = EIO;
        return -1;
    }
    return getrandom(buffer, length, 0) == (ssize_t)length ? 0 : -1;
}

// 4. Fallback stubs for kernel 4.x+ syscalls not present in Linux 3.0.35
int statx(int dirfd, const char *pathname, int flags, unsigned int mask, void *statxbuf) {
    errno = ENOSYS;
    return -1;
}

int renameat2(int olddirfd, const char *oldpath, int newdirfd, const char *newpath, unsigned int flags) {
    errno = ENOSYS;
    return -1;
}
