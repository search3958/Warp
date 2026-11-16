#ifndef MB5_VM_H
#define MB5_VM_H
#include <stddef.h>

typedef struct MB5Vm MB5Vm;

MB5Vm *mb5_vm_new(void);
void mb5_vm_free(MB5Vm *vm);
int mb5_vm_eval(MB5Vm *vm, const char *source);
int mb5_vm_call(MB5Vm *vm, const char *name, const char *argument);
int mb5_vm_has_proc(MB5Vm *vm, const char *name);
const char *mb5_vm_result(MB5Vm *vm);

/* Implemented by the Rust host. All strings are NUL-terminated UTF-8. */
int mb5_script_fetch(const char *method, const char *url, const char *body,
                     const char *content_type, const char *headers,
                     char *out, size_t out_cap, int *http_status);
int mb5_script_ui_set(const char *component_id, const char *property,
                      const char *value);
int mb5_script_ui_get(const char *component_id, const char *property,
                      char *out, size_t out_cap);
int mb5_script_schedule(const char *procedure, const char *argument,
                        unsigned long long delay_ms);
void mb5_script_log(const char *message);

#endif
