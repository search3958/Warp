#include "../src/scriptvm/mb5_vm.h"
#include <stdio.h>
#include <string.h>

static char ui_value[128];
static char ui_text[128];

int mb5_script_fetch(const char *method, const char *url, const char *body,
                     const char *content_type, const char *headers,
                     char *out, size_t out_cap, int *http_status) {
    (void)body; (void)content_type;
    if (strcmp(method, "GET") != 0 || strcmp(url, "https://example.test/data") != 0 ||
        strstr(headers, "X-Test: true") == NULL) return -1;
    const char *json = "{\"title\":\"MosaicBoard\",\"completed\":true,\"meta\":{\"version\":2},\"items\":[{\"name\":\"first\"},{\"name\":\"second\"}]}";
    if (strlen(json) + 1 > out_cap) return -1;
    strcpy(out, json);
    *http_status = 200;
    return 0;
}

int mb5_script_ui_set(const char *id, const char *property, const char *value) {
    if (strcmp(id, "status") != 0 || strcmp(property, "text") != 0) return 0;
    snprintf(ui_text, sizeof(ui_text), "%s", value);
    return 1;
}

int mb5_script_ui_get(const char *id, const char *property, char *out, size_t cap) {
    if (strcmp(id, "status") != 0 || strcmp(property, "value") != 0 || cap <= strlen(ui_value)) return 0;
    strcpy(out, ui_value);
    return 1;
}

int mb5_script_schedule(const char *procedure, const char *argument, unsigned long long delay_ms) {
    if (strcmp(procedure, "deferred") != 0 || strcmp(argument, "payload") != 0 || delay_ms != 25) return 0;
    return 42;
}

void mb5_script_log(const char *message) { fprintf(stderr, "[test] %s\n", message); }

int main(void) {
    static const char source[] =
        "set BOOTSTRAP initialized\n"
        "proc addOne {value} {\n"
        "  return [expr {$value + 1}]\n"
        "}\n"
        "proc main {} {\n"
        "  set booted [string equal $BOOTSTRAP initialized]\n"
        "  assert {$booted} {top-level initialization failed}\n"
        "  set count 0\n"
        "  while {$count < 4} { incr count }\n"
        "  set answer [addOne $count]\n"
        "  assert {$answer == 5} {math failed}\n"
        "  set words [list alpha {two words} gamma]\n"
        "  assert {[llength $words] == 3} {list failed}\n"
        "}\n";
    MB5Vm *vm = mb5_vm_new();
    if (!vm) return 10;
    if (mb5_vm_eval(vm, source) != 0) { fprintf(stderr, "VM parse/eval: %s\n", mb5_vm_result(vm)); return 11; }
    if (!mb5_vm_has_proc(vm, "main")) return 12;
    mb5_vm_free(vm);

    static const char integration[] =
        "proc deferred {payload} { ui.set status text $payload }\n"
        "proc scopeProbe {} { set shared local; return $shared }\n"
        "proc main {} {\n"
        "  set shared outside\n"
        "  set scoped [scopeProbe]\n"
        "  set localMatch [string equal $scoped local]\n"
        "  set globalMatch [string equal $shared outside]\n"
        "  assert {$localMatch} {local scope failed}\n"
        "  assert {$globalMatch} {local assignment leaked}\n"
        "  global APP_STATE\n"
        "  set APP_STATE ready\n"
        "  set globalReady [string equal $APP_STATE ready]\n"
        "  assert {$globalReady} {global assignment failed}\n"
        "  set task [after 25 deferred payload]\n"
        "  assert {$task == 42} {after scheduling failed}\n"
        "  set headers {X-Test: true}\n"
        "  set response [fetch GET https://example.test/data {} application/json $headers]\n"
        "  assert {$HTTP_STATUS == 200} {fetch status failed}\n"
        "  set title [json.get $response title]\n"
        "  set hasCompleted [json.has $response completed]\n"
        "  assert {$hasCompleted} {JSON key test failed}\n"
        "  set nestedVersion [json.get $response {meta.version}]\n"
        "  assert {$nestedVersion == 2} {nested JSON path failed}\n"
        "  set secondItem [json.get $response {items[1].name}]\n"
        "  set secondMatch [string equal $secondItem second]\n"
        "  assert {$secondMatch} {JSON array path failed}\n"
        "  set hasVersion [json.has $response {meta.version}]\n"
        "  assert {$hasVersion} {nested JSON has failed}\n"
        "  ui.set status text $title\n"
        "  set dict [dict create title $title count 3]\n"
        "  set dtitle [dict get $dict title]\n"
        "  if [string contains $dtitle Mosaic] { set result ok } else { error dictionary failed }\n"
        "  set captured {}\n"
        "  foreach item [list one two three] { lappend captured $item }\n"
        "  set capturedCount [llength $captured]\n"
        "  assert {$capturedCount == 3} {foreach failed}\n"
        "  set catchStatus [catch {error expected} caught]\n"
        "  assert {$catchStatus == 1} {catch failed}\n"
        "}\n";
    vm = mb5_vm_new();
    if (!vm) return 13;
    if (mb5_vm_eval(vm, integration) != 0) { fprintf(stderr, "integration parse/eval: %s\n", mb5_vm_result(vm)); return 14; }
    if (mb5_vm_call(vm, "main", "") != 0) { fprintf(stderr, "integration run: %s\n", mb5_vm_result(vm)); return 15; }
    if (strcmp(ui_text, "MosaicBoard") != 0) { fprintf(stderr, "UI bridge failed: %s\n", ui_text); return 16; }
    mb5_vm_free(vm);

    // Exercise nested evaluator/procedure frames. The interpreter now puts
    // variable-size command and procedure frames on the heap, not the native
    // stack, so recursion does not multiply 100+ KiB stack frames.
    static const char nested_source[] =
        "proc descend {remaining} {\n"
        "  if {$remaining > 0} {\n"
        "    set next [expr {$remaining - 1}]\n"
        "    descend $next\n"
        "  }\n"
        "}\n"
        "proc nestedMain {} { descend 8 }\n";
    vm = mb5_vm_new();
    if (!vm) return 17;
    if (mb5_vm_eval(vm, nested_source) != 0) {
        fprintf(stderr, "nested VM setup: %s\n", mb5_vm_result(vm));
        return 18;
    }
    if (mb5_vm_call(vm, "nestedMain", "") != 0) {
        fprintf(stderr, "nested VM execution: %s\n", mb5_vm_result(vm));
        return 19;
    }
    mb5_vm_free(vm);
    puts("MB5 VM tests passed");
    return 0;
}
