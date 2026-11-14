#define _GNU_SOURCE
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <stddef.h>
#include <sys/types.h>
#include <sys/syscall.h>
#include <stdarg.h>

// 1. Math functions wrapped to use double-precision GLIBC_2.4 routines in libm
double exp(double);
double log(double);
double pow(double, double);
double exp2(double);

float __wrap_expf(float x) {
    return (float)exp((double)x);
}

float __wrap_logf(float x) {
    return (float)log((double)x);
}

float __wrap_powf(float x, float y) {
    return (float)pow((double)x, (double)y);
}

float __wrap_exp2f(float x) {
    return (float)exp2((double)x);
}

// 2. Wrap fcntl and fcntl64 to use direct kernel syscalls
// avoiding glibc 2.28 symbol version bump
int __wrap_fcntl(int fd, int cmd, ...) {
    va_list va;
    va_start(va, cmd);
    void *arg = va_arg(va, void*);
    va_end(va);
    return syscall(__NR_fcntl, fd, cmd, arg);
}

int __wrap_fcntl64(int fd, int cmd, ...) {
    va_list va;
    va_start(va, cmd);
    void *arg = va_arg(va, void*);
    va_end(va);
#ifdef __NR_fcntl64
    return syscall(__NR_fcntl64, fd, cmd, arg);
#else
    return syscall(__NR_fcntl, fd, cmd, arg);
#endif
}

// 3. Wrap copy_file_range (introduced in GLIBC 2.27, kernel 4.5)
// Kindle OS 5.14 kernel 3.0.35 does not support this syscall.
// Returning ENOSYS prompts Rust std::io::copy to fall back to regular read/write loop.
loff_t __wrap_copy_file_range(int fd_in, loff_t *off_in, int fd_out, loff_t *off_out, size_t len, unsigned int flags) {
    errno = ENOSYS;
    return -1;
}

// 4. Provide local getrandom & getentropy implementation
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

// 5. Fallback stubs for kernel 4.x+ syscalls not present in Linux 3.0.35
int statx(int dirfd, const char *pathname, int flags, unsigned int mask, void *statxbuf) {
    errno = ENOSYS;
    return -1;
}

int renameat2(int olddirfd, const char *oldpath, int newdirfd, const char *newpath, unsigned int flags) {
    errno = ENOSYS;
    return -1;
}
