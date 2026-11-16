#include "../src/scriptvm/mb5_vm.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int mb5_script_fetch(const char *method, const char *url, const char *body,
                     const char *content_type, const char *headers,
                     char *out, size_t out_cap, int *http_status) {
    (void)method; (void)url; (void)body; (void)content_type; (void)headers;
    if (out_cap) out[0] = '\0';
    if (http_status) *http_status = 0;
    return -1;
}
int mb5_script_ui_set(const char *id, const char *property, const char *value) {
    (void)id; (void)property; (void)value; return 1;
}
int mb5_script_ui_get(const char *id, const char *property, char *out, size_t out_cap) {
    const char *value = "";
    if (strcmp(id, "show-details") == 0 && strcmp(property, "checked") == 0) value = "true";
    else if (strcmp(id, "quiet-mode") == 0 && strcmp(property, "checked") == 0) value = "false";
    else if (strcmp(id, "scale-control") == 0 && strcmp(property, "value") == 0) value = "68";
    else if (strcmp(id, "search-field") == 0 && strcmp(property, "value") == 0) value = "MosaicBoard";
    if (!out || strlen(value) + 1 > out_cap) return 0;
    strcpy(out, value); return 1;
}
int mb5_script_schedule(const char *procedure, const char *argument, unsigned long long delay_ms) {
    (void)argument; (void)delay_ms;
    return procedure && procedure[0] ? 1 : 0;
}
void mb5_script_log(const char *message) { (void)message; }

int main(int argc, char **argv) {
    if (argc != 2) { fprintf(stderr, "usage: script_vm_check path/to/script.mb5\n"); return 2; }
    FILE *file = fopen(argv[1], "rb");
    if (!file) { perror("fopen"); return 2; }
    char *source = (char *)malloc(32769);
    if (!source) { fclose(file); fprintf(stderr, "out of memory\n"); return 2; }
    size_t length = fread(source, 1, 32768, file);
    int read_error = ferror(file);
    int too_long = !feof(file);
    fclose(file);
    if (read_error || too_long) {
        fprintf(stderr, "script read failed or exceeds 32768 bytes\n");
        free(source); return 2;
    }
    source[length] = '\0';
    MB5Vm *vm = mb5_vm_new();
    if (!vm) { fprintf(stderr, "VM allocation failed\n"); free(source); return 2; }
    int status = mb5_vm_eval(vm, source);
    if (status == 0 && mb5_vm_has_proc(vm, "main")) {
        status = mb5_vm_call(vm, "main", "");
    }
    const char *smoke_tests[] = {
        "showWelcome", "searchChanged", "toggleDetails", "toggleQuietMode",
        "scaleChanged", "loadRemote", "sendSamplePost", "showNotice",
        "hideNotice", "changeSurface", "resetDemo", "showOverview",
        "showControls", "showNetwork", "chooseCompact",
        "chooseComfortable", "chooseSpacious", "showDesign",
        "showLogic", "showRuntime"
    };
    for (size_t i = 0; status == 0 && i < sizeof(smoke_tests) / sizeof(smoke_tests[0]); ++i) {
        if (mb5_vm_has_proc(vm, smoke_tests[i])) {
            status = mb5_vm_call(vm, smoke_tests[i], "");
            if (status != 0) fprintf(stderr, "callback %s failed: %s\n", smoke_tests[i], mb5_vm_result(vm));
        }
    }
    if (status != 0) fprintf(stderr, "script execution failed: %s\n", mb5_vm_result(vm));
    else printf("Script syntax and callback smoke checks passed: %s\n", argv[1]);
    mb5_vm_free(vm);
    free(source);
    return status == 0 ? 0 : 1;
}
