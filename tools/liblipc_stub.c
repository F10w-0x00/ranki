// Stub implementation of Amazon Kindle LIPC for cross-compilation linkage
#include <stddef.h>

void* LipcOpenNoName(void) { return NULL; }
void* LipcOpen(const char* name) { return NULL; }
void LipcClose(void* h) {}
int LipcGetIntProperty(void* h, const char* s, const char* p, int* v) { if (v) *v = 0; return -1; }
int LipcSetIntProperty(void* h, const char* s, const char* p, int v) { return -1; }
int LipcGetStringProperty(void* h, const char* s, const char* p, char** v) { if (v) *v = NULL; return -1; }
int LipcSetStringProperty(void* h, const char* s, const char* p, const char* v) { return -1; }
void LipcFreeString(char* s) {}
int LipcSubscribeExt(void* h, const char* s, const char* p, void* cb, void* data) { return -1; }
int LipcUnsubscribeExt(void* h, const char* s, const char* p, void* cb, void* data) { return -1; }
int LipcGetIntParam(void* e, const char* p, int* v) { if (v) *v = 0; return -1; }
int LipcGetStringParam(void* e, const char* p, char** v) { if (v) *v = NULL; return -1; }
