#define _POSIX_C_SOURCE 200809L
/*
 * Mosaic Board 5 Script VM
 *
 * A compact Tcl/Picol-shaped interpreter written for MB5 embedding.
 * This is an original implementation, not a copy of Picol.
 *
 * Grammar is command-oriented: words are separated by whitespace, commands
 * by newline/semicolon, braces create literal words, quotes interpolate,
 * $name substitutes variables and [command] substitutes command results.
 */
#include "mb5_vm.h"
#include <ctype.h>
#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define MB5_MAX_VARS 96
#define MB5_MAX_LOCAL_VARS 32
#define MB5_MAX_PROCS 48
#define MB5_MAX_ARGS 24
#define MB5_MAX_WORD 4096
#define MB5_MAX_RESULT 16384
#define MB5_MAX_BODY 8192
#define MB5_MAX_RECURSION 32
#define MB5_MAX_LOOP 10000
#define MB5_MAX_DICT 48

enum { VM_OK = 0, VM_ERROR = 1, VM_RETURN = 2, VM_BREAK = 3, VM_CONTINUE = 4 };
enum { WORD_PLAIN = 0, WORD_BLOCK = 1 };

typedef struct { char name[64]; char value[MB5_MAX_WORD]; } Mb5Var;
typedef struct Mb5Frame { Mb5Var vars[MB5_MAX_LOCAL_VARS]; int count; char global_names[MB5_MAX_LOCAL_VARS][64]; int global_count; struct Mb5Frame *parent; } Mb5Frame;
typedef struct { char name[64]; char args[MB5_MAX_ARGS][64]; int arg_count; char body[MB5_MAX_BODY]; } Mb5Proc;
typedef struct { char text[MB5_MAX_WORD]; int kind; } Mb5Word;

struct MB5Vm {
    Mb5Var globals[MB5_MAX_VARS];
    int global_count;
    Mb5Proc procs[MB5_MAX_PROCS];
    int proc_count;
    Mb5Frame *frame;
    char result[MB5_MAX_RESULT];
    int recursion;
    int last_status;
    int http_status;
};

static void set_result(MB5Vm *vm, const char *s) {
    if (!vm) return;
    if (!s) s = "";
    snprintf(vm->result, sizeof(vm->result), "%s", s);
}

static int is_global_name(const char *name) {
    return name && name[0] && isupper((unsigned char)name[0]);
}

static Mb5Var *find_var_in(Mb5Var *vars, int count, const char *name) {
    for (int i = 0; i < count; ++i) if (strcmp(vars[i].name, name) == 0) return &vars[i];
    return NULL;
}

static int declared_global(MB5Vm *vm, const char *name) {
    for (Mb5Frame *f = vm->frame; f; f = f->parent) {
        for (int i = 0; i < f->global_count; ++i) {
            if (strcmp(f->global_names[i], name) == 0) return 1;
        }
    }
    return 0;
}

static int targets_global(MB5Vm *vm, const char *name) {
    return is_global_name(name) || declared_global(vm, name);
}

static Mb5Var *get_var(MB5Vm *vm, const char *name) {
    if (targets_global(vm, name)) return find_var_in(vm->globals, vm->global_count, name);
    for (Mb5Frame *f = vm->frame; f; f = f->parent) {
        Mb5Var *v = find_var_in(f->vars, f->count, name);
        if (v) return v;
    }
    return find_var_in(vm->globals, vm->global_count, name);
}

static int set_var(MB5Vm *vm, const char *name, const char *value) {
    if (!name || !*name || strlen(name) >= 64 || !value) return 0;
    int global = targets_global(vm, name) || !vm->frame;
    Mb5Var *vars = global ? vm->globals : vm->frame->vars;
    int *count = global ? &vm->global_count : &vm->frame->count;
    int cap = global ? MB5_MAX_VARS : MB5_MAX_LOCAL_VARS;
    Mb5Var *v = find_var_in(vars, *count, name);
    if (!v) {
        if (*count >= cap) return 0;
        v = &vars[(*count)++];
        memset(v, 0, sizeof(*v));
        snprintf(v->name, sizeof(v->name), "%s", name);
    }
    int written = snprintf(v->value, sizeof(v->value), "%s", value);
    return written >= 0 && (size_t)written < sizeof(v->value);
}

static Mb5Proc *find_proc(MB5Vm *vm, const char *name) {
    for (int i = 0; i < vm->proc_count; ++i) if (strcmp(vm->procs[i].name, name) == 0) return &vm->procs[i];
    return NULL;
}

static void skip_space(const char **p) {
    while (**p == ' ' || **p == '\t' || **p == '\r') ++*p;
}

static int append_char(char *out, size_t cap, size_t *len, char c) {
    if (*len + 1 >= cap) return 0;
    out[(*len)++] = c; out[*len] = '\0'; return 1;
}

static int append_text(char *out, size_t cap, size_t *len, const char *s) {
    size_t n = strlen(s);
    if (*len + n >= cap) return 0;
    memcpy(out + *len, s, n); *len += n; out[*len] = '\0'; return 1;
}

static int eval_script_depth(MB5Vm *vm, const char *source, int depth);

static int expand_variable(MB5Vm *vm, const char **p, char *out, size_t cap, size_t *len) {
    char name[64]; size_t n = 0;
    ++*p;
    if (**p == '{') {
        ++*p;
        while (**p && **p != '}' && n < sizeof(name)-1) name[n++] = *(*p)++;
        if (**p == '}') ++*p;
    } else {
        while (**p && (isalnum((unsigned char)**p) || **p == '_' || **p == ':') && n < sizeof(name)-1) name[n++] = *(*p)++;
    }
    name[n] = '\0';
    if (!n) return append_char(out, cap, len, '$');
    Mb5Var *v = get_var(vm, name);
    if (!v) { set_result(vm, "unknown variable"); return 0; }
    return append_text(out, cap, len, v->value);
}

static int read_balanced(const char **p, char open, char close, char *out, size_t cap) {
    if (**p != open) return 0;
    ++*p;
    int level = 1, quote = 0, escaped = 0;
    size_t len = 0; out[0] = '\0';
    while (**p) {
        char c = *(*p)++;
        if (escaped) { if (!append_char(out, cap, &len, c)) return 0; escaped = 0; continue; }
        if (c == '\\') { escaped = 1; if (!append_char(out, cap, &len, c)) return 0; continue; }
        if (open == '{' && c == '"') quote = !quote;
        if (!quote && c == open) { ++level; if (level > 1 && !append_char(out, cap, &len, c)) return 0; continue; }
        if (!quote && c == close) {
            if (--level == 0) return 1;
            if (!append_char(out, cap, &len, c)) return 0;
            continue;
        }
        if (!append_char(out, cap, &len, c)) return 0;
    }
    return 0;
}

static int read_word(MB5Vm *vm, const char **p, Mb5Word *word, int depth) {
    skip_space(p);
    word->text[0] = '\0'; word->kind = WORD_PLAIN;
    if (!**p || **p == '\n' || **p == ';') return 0;
    if (**p == '{') {
        word->kind = WORD_BLOCK;
        return read_balanced(p, '{', '}', word->text, sizeof(word->text));
    }
    size_t len = 0;
    int quoted = 0;
    if (**p == '"') { quoted = 1; ++*p; }
    while (**p) {
        char c = **p;
        if (quoted && c == '"') { ++*p; return 1; }
        if (!quoted && (c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == ';')) break;
        if (c == '\\' && (*p)[1]) {
            ++*p; c = *(*p)++;
            switch (c) { case 'n': c='\n'; break; case 'r': c='\r'; break; case 't': c='\t'; break; default: break; }
            if (!append_char(word->text, sizeof(word->text), &len, c)) return 0;
            continue;
        }
        if (c == '$') {
            if (!expand_variable(vm, p, word->text, sizeof(word->text), &len)) return 0;
            continue;
        }
        if (c == '[') {
            char command[MB5_MAX_WORD];
            if (!read_balanced(p, '[', ']', command, sizeof(command))) return 0;
            int rc = eval_script_depth(vm, command, depth + 1);
            if (rc != VM_OK) return 0;
            if (!append_text(word->text, sizeof(word->text), &len, vm->result)) return 0;
            continue;
        }
        ++*p;
        if (!append_char(word->text, sizeof(word->text), &len, c)) return 0;
    }
    if (quoted) { set_result(vm, "unterminated quoted string"); return 0; }
    return 1;
}

/* Numeric expression parser. */
static const char *expr_p;
static int expr_error;
static void expr_ws(void) { while (*expr_p == ' ' || *expr_p == '\t') ++expr_p; }
static double expr_parse(int min_prec);
static double expr_primary(void) {
    expr_ws();
    if (*expr_p == '(') { ++expr_p; double v=expr_parse(0); expr_ws(); if (*expr_p==')') ++expr_p; else expr_error=1; return v; }
    if (*expr_p == '!') { ++expr_p; return !expr_primary(); }
    if (*expr_p == '-') { ++expr_p; return -expr_primary(); }
    if (strncmp(expr_p,"true",4)==0) { expr_p+=4; return 1; }
    if (strncmp(expr_p,"false",5)==0 || strncmp(expr_p,"nil",3)==0) { expr_p += (*expr_p=='f'?5:3); return 0; }
    char *end = NULL; errno = 0; double v = strtod(expr_p, &end);
    if (end == expr_p || errno == ERANGE) { expr_error=1; return 0; }
    expr_p=end; return v;
}
static int expr_precedence(const char **op, int *length) {
    expr_ws();
    const char *s=expr_p; *length=1;
    if (!*s) return -1;
    if (s[0]=='|' && s[1]=='|') { *op="||"; *length=2; return 1; }
    if (s[0]=='&' && s[1]=='&') { *op="&&"; *length=2; return 2; }
    if ((s[0]=='=' && s[1]=='=') || (s[0]=='!' && s[1]=='=') || (s[0]=='<' && s[1]=='=') || (s[0]=='>' && s[1]=='=')) { *op=s; *length=2; return 3; }
    if (s[0]=='<' || s[0]=='>') { *op=s; return 3; }
    if (s[0]=='+' || s[0]=='-') { *op=s; return 4; }
    if (s[0]=='*' || s[0]=='/') { *op=s; return 5; }
    return -1;
}
static double expr_parse(int min_prec) {
    double left=expr_primary();
    while (!expr_error) {
        const char *op=NULL; int len=0; int prec=expr_precedence(&op,&len);
        if (prec < min_prec) break;
        expr_p += len;
        double right=expr_parse(prec+1);
        if (!strcmp(op,"||")) left=(left!=0 || right!=0);
        else if (!strcmp(op,"&&")) left=(left!=0 && right!=0);
        else if (!strcmp(op,"==")) left=(left==right);
        else if (!strcmp(op,"!=")) left=(left!=right);
        else if (*op=='<' && len==2) left=(left<=right);
        else if (*op=='>' && len==2) left=(left>=right);
        else if (*op=='<') left=(left<right);
        else if (*op=='>') left=(left>right);
        else if (*op=='+') left+=right;
        else if (*op=='-') left-=right;
        else if (*op=='*') left*=right;
        else if (*op=='/') { if (right==0) expr_error=1; else left/=right; }
    }
    return left;
}
static int evaluate_number(MB5Vm *vm, const char *s, double *value) {
    char expanded[MB5_MAX_WORD]; size_t len=0; expanded[0]='\0';
    const char *p=s;
    while (*p) {
        if (*p=='$') { if(!expand_variable(vm,&p,expanded,sizeof(expanded),&len)) return 0; }
        else if (!append_char(expanded,sizeof(expanded),&len,*p++)) return 0;
    }
    expr_p=expanded; expr_error=0; double v=expr_parse(0); expr_ws();
    if (expr_error || *expr_p!='\0') return 0;
    *value=v;
    return 1;
}
static int evaluate_condition(MB5Vm *vm, const char *s, int *ok) {
    double value=0;
    *ok=evaluate_number(vm,s,&value);
    return *ok && value != 0.0;
}

static int eval_command(MB5Vm *vm, Mb5Word *w, int argc, int depth);

static int eval_script_depth(MB5Vm *vm, const char *source, int depth) {
    if (!vm || !source) return VM_ERROR;
    if (depth > MB5_MAX_RECURSION) { set_result(vm,"script recursion limit exceeded"); return VM_ERROR; }
    if (++vm->recursion > MB5_MAX_RECURSION) { --vm->recursion; set_result(vm,"script recursion limit exceeded"); return VM_ERROR; }
    const char *p=source;
    int rc=VM_OK;
    // Each command token can hold up to MB5_MAX_WORD bytes. Allocate this
    // scratch space on the heap instead of consuming ~96 KiB per recursive
    // evaluator frame on the platform's comparatively small thread stack.
    Mb5Word *words=(Mb5Word*)calloc(MB5_MAX_ARGS,sizeof(*words));
    if (!words) { --vm->recursion; set_result(vm,"script evaluator ran out of memory"); return VM_ERROR; }
    while (*p) {
        while (*p==' ' || *p=='\t' || *p=='\r' || *p=='\n' || *p==';') ++p;
        if (!*p) break;
        if (*p=='#') { while (*p && *p!='\n') ++p; continue; }
        int argc=0;
        while (*p && *p!='\n' && *p!=';') {
            if (argc>=MB5_MAX_ARGS) { set_result(vm,"too many command arguments"); rc=VM_ERROR; goto done; }
            if (!read_word(vm,&p,&words[argc],depth)) { if (!vm->result[0]) set_result(vm,"invalid command word"); rc=VM_ERROR; goto done; }
            ++argc;
            while (*p==' ' || *p=='\t' || *p=='\r') ++p;
        }
        if (*p=='\n' || *p==';') ++p;
        if (argc) { rc=eval_command(vm,words,argc,depth); if (rc!=VM_OK) goto done; }
    }
done:
    free(words);
    --vm->recursion;
    return rc;
}

static int eval_proc(MB5Vm *vm, Mb5Proc *proc, Mb5Word *words, int argc, int depth) {
    if (argc - 1 != proc->arg_count) { set_result(vm,"wrong number of procedure arguments"); return VM_ERROR; }
    // A local frame contains 32 fixed-size variables (~130 KiB). Keep nested
    // procedure calls off the native stack and always restore the parent.
    Mb5Frame *local=(Mb5Frame*)calloc(1,sizeof(*local));
    if (!local) { set_result(vm,"procedure frame allocation failed"); return VM_ERROR; }
    local->parent=vm->frame; vm->frame=local;
    for (int i=0;i<proc->arg_count;i++) {
        if (!set_var(vm,proc->args[i],words[i+1].text)) {
            vm->frame=local->parent; free(local);
            set_result(vm,"procedure variable capacity exceeded"); return VM_ERROR;
        }
    }
    int rc=eval_script_depth(vm,proc->body,depth+1);
    vm->frame=local->parent;
    free(local);
    if (rc==VM_RETURN) return VM_OK;
    return rc;
}

static int parse_proc(MB5Vm *vm, Mb5Word *w, int argc) {
    if (argc!=4) { set_result(vm,"proc name {args} {body} expected"); return VM_ERROR; }
    if (vm->proc_count>=MB5_MAX_PROCS || !w[1].text[0]) { set_result(vm,"procedure capacity exceeded"); return VM_ERROR; }
    Mb5Proc *p=find_proc(vm,w[1].text);
    if (!p) p=&vm->procs[vm->proc_count++];
    memset(p,0,sizeof(*p)); memcpy(p->name,w[1].text,strlen(w[1].text)<sizeof(p->name)-1?strlen(w[1].text):sizeof(p->name)-1);
    char args[MB5_MAX_WORD]; snprintf(args,sizeof(args),"%s",w[2].text);
    char *save=NULL; char *token=strtok_r(args," \t\r\n,",&save);
    while (token && p->arg_count<MB5_MAX_ARGS) { snprintf(p->args[p->arg_count++],64,"%s",token); token=strtok_r(NULL," \t\r\n,",&save); }
    snprintf(p->body,sizeof(p->body),"%s",w[3].text);
    set_result(vm,""); return VM_OK;
}

static int list_word(const char *list, int wanted, char *out, size_t cap) {
    const char *p=list; int index=0;
    while (*p) {
        while (*p==' ') ++p;
        if (!*p) break;
        char temp[MB5_MAX_WORD]; size_t n=0;
        if (*p=='{') { ++p; int level=1; while (*p && level) { char c=*p++; if(c=='{')++level; else if(c=='}'){if(--level==0)break;} if(level && n+1<sizeof(temp))temp[n++]=c; } }
        else { while(*p && *p!=' ') { if(n+1<sizeof(temp))temp[n++]=*p; ++p; } }
        temp[n]='\0';
        if(index++==wanted) { snprintf(out,cap,"%s",temp); return 1; }
    }
    return 0;
}

static void json_ws(const char **p) {
    while (**p && isspace((unsigned char)**p)) ++*p;
}

static int json_string_end(const char **p) {
    if (**p != '"') return 0;
    ++*p;
    while (**p) {
        if (**p == '\\') { if (!(*p)[1]) return 0; *p += 2; continue; }
        if (**p == '"') { ++*p; return 1; }
        ++*p;
    }
    return 0;
}

static int json_value_end(const char **p, int depth) {
    if (depth > 24) return 0;
    json_ws(p);
    if (**p == '"') return json_string_end(p);
    if (**p == '{' || **p == '[') {
        char stack[25]; int level = 0; int in_string = 0;
        stack[level++] = **p == '{' ? '}' : ']'; ++*p;
        while (**p && level > 0) {
            char ch = **p;
            if (in_string) {
                if (ch == '\\') { if (!(*p)[1]) return 0; *p += 2; continue; }
                if (ch == '"') in_string = 0;
            } else if (ch == '"') {
                in_string = 1;
            } else if (ch == '{' || ch == '[') {
                if (level >= 24) return 0;
                stack[level++] = ch == '{' ? '}' : ']';
            } else if (ch == '}' || ch == ']') {
                if (stack[level - 1] != ch) return 0;
                --level;
            }
            ++*p;
        }
        return level == 0;
    }
    const char *start = *p;
    while (**p && **p != ',' && **p != '}' && **p != ']' && !isspace((unsigned char)**p)) ++*p;
    return *p > start;
}

static int json_unescape_string(const char *src, size_t length, char *out, size_t cap) {
    if (length < 2 || src[0] != '"' || src[length - 1] != '"' || cap == 0) return 0;
    size_t used = 0;
    for (size_t i = 1; i + 1 < length; ++i) {
        char ch = src[i];
        if (ch == '\\') {
            if (++i + 1 >= length) return 0;
            switch (src[i]) {
                case '"': ch = '"'; break;
                case '\\': ch = '\\'; break;
                case '/': ch = '/'; break;
                case 'b': ch = '\b'; break;
                case 'f': ch = '\f'; break;
                case 'n': ch = '\n'; break;
                case 'r': ch = '\r'; break;
                case 't': ch = '\t'; break;
                case 'u':
                    /* Preserve \uXXXX escapes as ASCII; full Unicode decoding is host-specific. */
                    if (used + 6 >= cap || i + 4 + 1 >= length) return 0;
                    out[used++] = '\\'; out[used++] = 'u';
                    for (int j = 0; j < 4; ++j) out[used++] = src[++i];
                    continue;
                default: return 0;
            }
        }
        if (used + 1 >= cap) return 0;
        out[used++] = ch;
    }
    out[used] = '\0';
    return 1;
}

static int json_member_raw(const char *json, const char *key, const char **value_start, size_t *value_len) {
    if (!json || !key || !value_start || !value_len) return 0;
    const char *p = json;
    json_ws(&p);
    if (*p++ != '{') return 0;
    json_ws(&p);
    while (*p && *p != '}') {
        const char *key_start = p;
        if (*p != '"' || !json_string_end(&p)) return 0;
        const size_t raw_key_len = (size_t)(p - key_start);
        char decoded_key[256];
        if (!json_unescape_string(key_start, raw_key_len, decoded_key, sizeof(decoded_key))) return 0;
        json_ws(&p);
        if (*p++ != ':') return 0;
        json_ws(&p);
        const char *start = p;
        if (!json_value_end(&p, 0)) return 0;
        if (strcmp(decoded_key, key) == 0) {
            *value_start = start;
            *value_len = (size_t)(p - start);
            return 1;
        }
        json_ws(&p);
        if (*p == ',') { ++p; json_ws(&p); continue; }
        if (*p == '}') break;
        return 0;
    }
    return 0;
}

static int json_array_raw(const char *json, size_t wanted, const char **value_start, size_t *value_len) {
    if (!json || !value_start || !value_len) return 0;
    const char *p = json;
    json_ws(&p);
    if (*p++ != '[') return 0;
    json_ws(&p);
    size_t index = 0;
    while (*p && *p != ']') {
        const char *start = p;
        if (!json_value_end(&p, 0)) return 0;
        if (index == wanted) {
            *value_start = start;
            *value_len = (size_t)(p - start);
            return 1;
        }
        ++index;
        json_ws(&p);
        if (*p == ',') { ++p; json_ws(&p); continue; }
        if (*p == ']') break;
        return 0;
    }
    return 0;
}

/* Supports dotted object paths and zero-based array indexes, e.g. user.name
 * and todos[0].title. Values that resolve to objects/arrays remain raw JSON. */
static int json_find_path(const char *json, const char *path, const char **value_start, size_t *value_len) {
    if (!json || !path || !*path || !value_start || !value_len) return 0;
    const char *cursor = json;
    const char *part = path;
    int found_any = 0;
    while (*part) {
        const char *next_value = NULL;
        size_t next_len = 0;
        if (*part == '.') ++part;
        if (!*part) return 0;
        if (*part == '[') {
            ++part;
            if (!isdigit((unsigned char)*part)) return 0;
            size_t index = 0;
            while (isdigit((unsigned char)*part)) {
                size_t digit = (size_t)(*part++ - '0');
                if (index > (SIZE_MAX - digit) / 10) return 0;
                index = index * 10 + digit;
            }
            if (*part++ != ']') return 0;
            if (!json_array_raw(cursor, index, &next_value, &next_len)) return 0;
        } else {
            char key[256];
            size_t key_len = 0;
            while (*part && *part != '.' && *part != '[') {
                if (key_len + 1 >= sizeof(key)) return 0;
                key[key_len++] = *part++;
            }
            if (key_len == 0) return 0;
            key[key_len] = '\0';
            if (!json_member_raw(cursor, key, &next_value, &next_len)) return 0;
        }
        cursor = next_value;
        *value_start = next_value;
        *value_len = next_len;
        found_any = 1;
        if (*part && *part != '.' && *part != '[') return 0;
    }
    return found_any;
}

static int json_find(const char *json, const char *path, char *out, size_t cap) {
    if (!json || !path || !out || cap == 0) return 0;
    const char *value_start = NULL;
    size_t value_len = 0;
    if (!json_find_path(json, path, &value_start, &value_len) || value_len >= cap) return 0;
    if (value_len >= 2 && value_start[0] == '"') {
        return json_unescape_string(value_start, value_len, out, cap);
    }
    memcpy(out, value_start, value_len);
    out[value_len] = '\0';
    return 1;
}

static int dict_set_value(const char *dict, const char *key, const char *value, char *out, size_t cap) {
    int found = 0, index = 0; char k[MB5_MAX_WORD], v[MB5_MAX_WORD]; size_t len = 0; out[0] = '\0';
    while (list_word(dict, index, k, sizeof(k)) && list_word(dict, index + 1, v, sizeof(v))) {
        const char *write_value = v;
        if (!strcmp(k, key)) { write_value = value; found = 1; }
        if (len && !append_char(out, cap, &len, ' ')) return 0;
        int brace_key = strpbrk(k, " \t\n{}") != NULL;
        int brace_value = strpbrk(write_value, " \t\n{}") != NULL;
        if (brace_key && !append_char(out, cap, &len, '{')) return 0;
        if (!append_text(out, cap, &len, k)) return 0;
        if (brace_key && !append_char(out, cap, &len, '}')) return 0;
        if (!append_char(out, cap, &len, ' ')) return 0;
        if (brace_value && !append_char(out, cap, &len, '{')) return 0;
        if (!append_text(out, cap, &len, write_value)) return 0;
        if (brace_value && !append_char(out, cap, &len, '}')) return 0;
        index += 2;
    }
    if (!found) {
        if (len && !append_char(out, cap, &len, ' ')) return 0;
        int brace_key = strpbrk(key, " \t\n{}") != NULL;
        int brace_value = strpbrk(value, " \t\n{}") != NULL;
        if (brace_key && !append_char(out, cap, &len, '{')) return 0;
        if (!append_text(out, cap, &len, key)) return 0;
        if (brace_key && !append_char(out, cap, &len, '}')) return 0;
        if (!append_char(out, cap, &len, ' ')) return 0;
        if (brace_value && !append_char(out, cap, &len, '{')) return 0;
        if (!append_text(out, cap, &len, value)) return 0;
        if (brace_value && !append_char(out, cap, &len, '}')) return 0;
    }
    return 1;
}

static int eval_command(MB5Vm *vm, Mb5Word *w, int argc, int depth) {
    const char *cmd=w[0].text;
    if (!strcmp(cmd,"proc") || !strcmp(cmd,"fn")) return parse_proc(vm,w,argc);
    if (!strcmp(cmd,"set") || !strcmp(cmd,"let")) {
        if (argc==2) { Mb5Var *v=get_var(vm,w[1].text); if(!v){set_result(vm,"unknown variable");return VM_ERROR;} set_result(vm,v->value); return VM_OK; }
        if (argc!=3 || !set_var(vm,w[1].text,w[2].text)) { set_result(vm,"set name value expected or variable capacity exceeded"); return VM_ERROR; }
        set_result(vm,w[2].text); return VM_OK;
    }
    if (!strcmp(cmd,"global")) {
        if (argc < 2 || !vm->frame) { set_result(vm,"global name ?name ...? is valid inside a procedure"); return VM_ERROR; }
        for (int i = 1; i < argc; ++i) {
            if (!w[i].text[0] || strlen(w[i].text) >= 64) { set_result(vm,"invalid global variable name"); return VM_ERROR; }
            int exists = 0;
            for (int n = 0; n < vm->frame->global_count; ++n) if (!strcmp(vm->frame->global_names[n],w[i].text)) exists = 1;
            if (!exists) {
                if (vm->frame->global_count >= MB5_MAX_LOCAL_VARS) { set_result(vm,"global declaration capacity exceeded"); return VM_ERROR; }
                size_t name_len = strlen(w[i].text); /* validated to be < 64 above */
                memcpy(vm->frame->global_names[vm->frame->global_count], w[i].text, name_len + 1);
                vm->frame->global_count++;
            }
        }
        set_result(vm,""); return VM_OK;
    }
    if (!strcmp(cmd,"expr")) {
        if (argc<2) { set_result(vm,"expr requires an expression"); return VM_ERROR; }
        char expr[MB5_MAX_WORD]=""; for(int i=1;i<argc;i++){ if(i>1)strncat(expr," ",sizeof(expr)-strlen(expr)-1); strncat(expr,w[i].text,sizeof(expr)-strlen(expr)-1); }
        double value=0; if(!evaluate_number(vm,expr,&value)){set_result(vm,"invalid expression");return VM_ERROR;} snprintf(vm->result,sizeof(vm->result),"%.12g",value); return VM_OK;
    }
    if (!strcmp(cmd,"if")) {
        if(argc<3){set_result(vm,"if condition body expected");return VM_ERROR;}
        int ok=0; int truth=evaluate_condition(vm,w[1].text,&ok); if(!ok){set_result(vm,"invalid if condition");return VM_ERROR;}
        if(truth)return eval_script_depth(vm,w[2].text,depth+1);
        int i=3; while(i<argc){ if(!strcmp(w[i].text,"else"))return (i+1<argc)?eval_script_depth(vm,w[i+1].text,depth+1):VM_ERROR; if(!strcmp(w[i].text,"elseif")&&i+2<argc){truth=evaluate_condition(vm,w[i+1].text,&ok);if(!ok){set_result(vm,"invalid elseif condition");return VM_ERROR;}if(truth)return eval_script_depth(vm,w[i+2].text,depth+1);i+=3;}else{set_result(vm,"invalid if branch");return VM_ERROR;} } set_result(vm,"");return VM_OK;
    }
    if (!strcmp(cmd,"while")) {
        if(argc!=3){set_result(vm,"while condition body expected");return VM_ERROR;}
        for(int loop=0;loop<MB5_MAX_LOOP;loop++){int ok=0;if(!evaluate_condition(vm,w[1].text,&ok)){if(!ok){set_result(vm,"invalid while condition");return VM_ERROR;}set_result(vm,"");return VM_OK;}int rc=eval_script_depth(vm,w[2].text,depth+1);if(rc==VM_BREAK){set_result(vm,"");return VM_OK;}if(rc==VM_CONTINUE||rc==VM_OK)continue;return rc;}set_result(vm,"while loop iteration limit exceeded");return VM_ERROR;
    }
    if (!strcmp(cmd,"return")) { if(argc>2){set_result(vm,"return accepts at most one value");return VM_ERROR;} set_result(vm,argc==2?w[1].text:"");return VM_RETURN; }
    if (!strcmp(cmd,"break")) return argc==1?VM_BREAK:VM_ERROR;
    if (!strcmp(cmd,"continue")) return argc==1?VM_CONTINUE:VM_ERROR;
    if (!strcmp(cmd,"error")) { set_result(vm, argc>1?w[1].text:"script error"); return VM_ERROR; }
    if (!strcmp(cmd,"assert")) { if(argc<2||argc>3){set_result(vm,"assert condition ?message?");return VM_ERROR;}int ok=0;if(!evaluate_condition(vm,w[1].text,&ok)){set_result(vm,argc==3?w[2].text:"assertion failed");return VM_ERROR;}set_result(vm,"1");return VM_OK; }
    if (!strcmp(cmd,"catch")) {
        if(argc<2||argc>3){set_result(vm,"catch script ?result-variable?");return VM_ERROR;}
        int caught=eval_script_depth(vm,w[1].text,depth+1); char saved[MB5_MAX_RESULT]; snprintf(saved,sizeof(saved),"%s",vm->result);
        if(caught==VM_RETURN||caught==VM_BREAK||caught==VM_CONTINUE)caught=VM_ERROR;
        if(argc==3 && !set_var(vm,w[2].text,saved)){set_result(vm,"catch result variable capacity exceeded");return VM_ERROR;}
        snprintf(vm->result,sizeof(vm->result),"%d",caught==VM_OK?0:1); return VM_OK;
    }
    if (!strcmp(cmd,"foreach")) {
        if(argc!=4){set_result(vm,"foreach variable list body expected");return VM_ERROR;}
        for(int n=0;n<MB5_MAX_LOOP;n++){
            char value[MB5_MAX_WORD];if(!list_word(w[2].text,n,value,sizeof(value))) {set_result(vm,"");return VM_OK;}
            if(!set_var(vm,w[1].text,value)){set_result(vm,"foreach variable capacity exceeded");return VM_ERROR;}
            int rc=eval_script_depth(vm,w[3].text,depth+1);if(rc==VM_BREAK){set_result(vm,"");return VM_OK;}if(rc==VM_CONTINUE)continue;if(rc!=VM_OK)return rc;
        }
        set_result(vm,"foreach loop iteration limit exceeded");return VM_ERROR;
    }
    if (!strcmp(cmd,"puts") || !strcmp(cmd,"print")) { if(argc<2){set_result(vm,"puts requires a value");return VM_ERROR;} char msg[MB5_MAX_WORD]="";for(int i=1;i<argc;i++){if(i>1)strncat(msg," ",sizeof(msg)-strlen(msg)-1);strncat(msg,w[i].text,sizeof(msg)-strlen(msg)-1);}mb5_script_log(msg);set_result(vm, msg);return VM_OK; }
    if (!strcmp(cmd,"incr")) { if(argc<2||argc>3){set_result(vm,"incr variable ?amount?");return VM_ERROR;}Mb5Var*v=get_var(vm,w[1].text);double a=v?atof(v->value):0;double b=argc==3?atof(w[2].text):1;char buf[64];snprintf(buf,sizeof(buf),"%.12g",a+b);if(!set_var(vm,w[1].text,buf)){set_result(vm,"variable capacity exceeded");return VM_ERROR;}set_result(vm, buf);return VM_OK; }
    if (!strcmp(cmd,"append")) {
        if(argc<3){set_result(vm,"append variable value ?value ...?");return VM_ERROR;}
        Mb5Var *v=get_var(vm,w[1].text);char buf[MB5_MAX_WORD]="";if(v)snprintf(buf,sizeof(buf),"%s",v->value);
        for(int i=2;i<argc;i++){if(strlen(buf)+strlen(w[i].text)>=sizeof(buf)-1){set_result(vm,"append value exceeds limit");return VM_ERROR;}strcat(buf,w[i].text);}
        if(!set_var(vm,w[1].text,buf)){set_result(vm,"variable capacity exceeded");return VM_ERROR;}set_result(vm,buf);return VM_OK;
    }
    if (!strcmp(cmd,"lappend")) {
        if(argc<3){set_result(vm,"lappend variable value ?value ...?");return VM_ERROR;}
        Mb5Var *v=get_var(vm,w[1].text);char buf[MB5_MAX_RESULT]="";size_t len=0;if(v)append_text(buf,sizeof(buf),&len,v->value);
        for(int i=2;i<argc;i++){if(len&&!append_char(buf,sizeof(buf),&len,' ')){set_result(vm,"list exceeds limit");return VM_ERROR;}int q=strpbrk(w[i].text," \t\n{}")!=NULL;if(q&&!append_char(buf,sizeof(buf),&len,'{')){set_result(vm,"list exceeds limit");return VM_ERROR;}if(!append_text(buf,sizeof(buf),&len,w[i].text)||(q&&!append_char(buf,sizeof(buf),&len,'}'))){set_result(vm,"list exceeds limit");return VM_ERROR;}}
        if(!set_var(vm,w[1].text,buf)){set_result(vm,"variable capacity exceeded");return VM_ERROR;}set_result(vm,buf);return VM_OK;
    }
    if (!strcmp(cmd,"list")) { char buf[MB5_MAX_RESULT]="";size_t len=0;for(int i=1;i<argc;i++){if(i>1)append_char(buf,sizeof(buf),&len,' ');int quote=strpbrk(w[i].text," \t\n{}")!=NULL;if(quote)append_char(buf,sizeof(buf),&len,'{');append_text(buf,sizeof(buf),&len,w[i].text);if(quote)append_char(buf,sizeof(buf),&len,'}');}set_result(vm,buf);return VM_OK; }
    if (!strcmp(cmd,"llength")) { if(argc!=2){set_result(vm,"llength list expected");return VM_ERROR;}int n=0;char temp[MB5_MAX_WORD];while(list_word(w[1].text,n,temp,sizeof(temp)))n++;snprintf(vm->result,sizeof(vm->result),"%d",n);return VM_OK; }
    if (!strcmp(cmd,"lindex")) { if(argc!=3){set_result(vm,"lindex list index expected");return VM_ERROR;}char out[MB5_MAX_WORD];if(!list_word(w[1].text,atoi(w[2].text),out,sizeof(out)))out[0]='\0';set_result(vm,out);return VM_OK; }
    if (!strcmp(cmd,"dict")) {
        if(argc<2){set_result(vm,"dict create/get/set/exists/keys expected");return VM_ERROR;}
        if(!strcmp(w[1].text,"create")){char out[MB5_MAX_RESULT]="";size_t len=0;for(int i=2;i<argc;i++){if(i>2)append_char(out,sizeof(out),&len,' ');int q=strpbrk(w[i].text," \t\n{}")!=NULL;if(q)append_char(out,sizeof(out),&len,'{');append_text(out,sizeof(out),&len,w[i].text);if(q)append_char(out,sizeof(out),&len,'}');}set_result(vm,out);return VM_OK;}
        if(!strcmp(w[1].text,"set")&&argc==5){Mb5Var *v=get_var(vm,w[2].text);char out[MB5_MAX_RESULT];if(!dict_set_value(v?v->value:"",w[3].text,w[4].text,out,sizeof(out))||!set_var(vm,w[2].text,out)){set_result(vm,"dictionary update failed");return VM_ERROR;}set_result(vm,out);return VM_OK;}
        if(!strcmp(w[1].text,"get")&&argc==4){int n=0;char k[MB5_MAX_WORD],v[MB5_MAX_WORD];while(list_word(w[2].text,n,k,sizeof(k))&&list_word(w[2].text,n+1,v,sizeof(v))){if(!strcmp(k,w[3].text)){set_result(vm,v);return VM_OK;}n+=2;}set_result(vm,"dictionary key not found");return VM_ERROR;}
        if(!strcmp(w[1].text,"exists")&&argc==4){int n=0;char k[MB5_MAX_WORD],v[MB5_MAX_WORD];while(list_word(w[2].text,n,k,sizeof(k))&&list_word(w[2].text,n+1,v,sizeof(v))){if(!strcmp(k,w[3].text)){set_result(vm,"1");return VM_OK;}n+=2;}set_result(vm,"0");return VM_OK;}
        if(!strcmp(w[1].text,"keys")&&argc==3){char out[MB5_MAX_RESULT]="";size_t len=0;int n=0;char k[MB5_MAX_WORD],v[MB5_MAX_WORD];while(list_word(w[2].text,n,k,sizeof(k))&&list_word(w[2].text,n+1,v,sizeof(v))){if(len)append_char(out,sizeof(out),&len,' ');append_text(out,sizeof(out),&len,k);n+=2;}set_result(vm,out);return VM_OK;}
        set_result(vm,"unsupported dict operation");return VM_ERROR;
    }
    if (!strcmp(cmd,"string")) {
        if(argc<3){set_result(vm,"string length/trim/contains/lower/upper value expected");return VM_ERROR;}
        if(!strcmp(w[1].text,"length")){snprintf(vm->result,sizeof(vm->result),"%lu",(unsigned long)strlen(w[2].text));return VM_OK;}
        if(!strcmp(w[1].text,"trim")){set_result(vm,w[2].text);char *a=vm->result;while(isspace((unsigned char)*a))++a;if(a!=vm->result)memmove(vm->result,a,strlen(a)+1);size_t n=strlen(vm->result);while(n&&isspace((unsigned char)vm->result[n-1]))vm->result[--n]='\0';return VM_OK;}
        if(!strcmp(w[1].text,"contains")&&argc==4){set_result(vm,strstr(w[2].text,w[3].text)?"1":"0");return VM_OK;}
        if(!strcmp(w[1].text,"equal")&&argc==4){set_result(vm,strcmp(w[2].text,w[3].text)==0?"1":"0");return VM_OK;}
        if(!strcmp(w[1].text,"startsWith")&&argc==4){size_t n=strlen(w[3].text);set_result(vm,strncmp(w[2].text,w[3].text,n)==0?"1":"0");return VM_OK;}
        if((!strcmp(w[1].text,"lower")||!strcmp(w[1].text,"upper"))&&argc==3){snprintf(vm->result,sizeof(vm->result),"%s",w[2].text);for(char *p=vm->result;*p;p++)*p=!strcmp(w[1].text,"lower")?(char)tolower((unsigned char)*p):(char)toupper((unsigned char)*p);return VM_OK;}
        set_result(vm,"unsupported string operation");return VM_ERROR;
    }
    if (!strcmp(cmd,"json.get")) { if(argc!=3){set_result(vm,"json.get body key expected");return VM_ERROR;}char out[MB5_MAX_WORD];if(!json_find(w[1].text,w[2].text,out,sizeof(out))){set_result(vm,"JSON key not found");return VM_ERROR;}set_result(vm,out);return VM_OK; }
    if (!strcmp(cmd,"json.has")) { if(argc!=3){set_result(vm,"json.has body key expected");return VM_ERROR;}char out[MB5_MAX_WORD];set_result(vm,json_find(w[1].text,w[2].text,out,sizeof(out))?"1":"0");return VM_OK; }
    if (!strcmp(cmd,"fetch")) {
        const char *method="GET", *url=NULL, *body="", *ctype="application/json", *headers="";
        if(argc==2)url=w[1].text;
        else if(argc>=3){method=w[1].text;url=w[2].text;if(argc>=4)body=w[3].text;if(argc>=5)ctype=w[4].text;if(argc>=6)headers=w[5].text;if(argc>6){set_result(vm,"fetch accepts at most METHOD URL BODY CONTENT_TYPE HEADERS");return VM_ERROR;}}
        else{set_result(vm,"fetch url or fetch METHOD url ?body? ?content-type? ?headers?");return VM_ERROR;}
        char response[MB5_MAX_RESULT];int status=0;int rc=mb5_script_fetch(method,url,body,ctype,headers,response,sizeof(response),&status);vm->http_status=status;char stat[16];snprintf(stat,sizeof(stat),"%d",status);set_var(vm,"HTTP_STATUS",stat);if(rc<0){set_var(vm,"HTTP_ERROR","request failed or response exceeded the limit");set_result(vm,"fetch failed");return VM_ERROR;}set_var(vm,"HTTP_ERROR",status==0?"network request did not return an HTTP status":"");if(status==0){set_result(vm,"fetch failed");return VM_ERROR;}set_result(vm,response);return VM_OK;
    }
    if (!strcmp(cmd,"ui.set")) { if(argc!=4){set_result(vm,"ui.set component-id property value expected");return VM_ERROR;}if(!mb5_script_ui_set(w[1].text,w[2].text,w[3].text)){set_result(vm,"UI target was not found or property is invalid");return VM_ERROR;}set_result(vm,w[3].text);return VM_OK; }
    if (!strcmp(cmd,"ui.get")) { if(argc!=3){set_result(vm,"ui.get component-id property expected");return VM_ERROR;}char out[MB5_MAX_WORD];if(!mb5_script_ui_get(w[1].text,w[2].text,out,sizeof(out))){set_result(vm,"UI value was not found");return VM_ERROR;}set_result(vm,out);return VM_OK; }
    if (!strcmp(cmd,"after")) {
        if (argc < 3 || argc > 4) { set_result(vm,"after milliseconds procedure ?argument?"); return VM_ERROR; }
        if (!find_proc(vm,w[2].text)) { set_result(vm,"after target procedure was not found"); return VM_ERROR; }
        char *end = NULL; errno = 0; unsigned long ms = strtoul(w[1].text,&end,10);
        if (errno == ERANGE || !end || *end || ms > 3600000UL) { set_result(vm,"after delay must be between 0 and 3600000ms"); return VM_ERROR; }
        int id = mb5_script_schedule(w[2].text,argc == 4 ? w[3].text : "",(unsigned long long)ms);
        if (id <= 0) { set_result(vm,"scheduled callback queue is full"); return VM_ERROR; }
        snprintf(vm->result,sizeof(vm->result),"%d",id); return VM_OK;
    }
    if (!strcmp(cmd,"wait")) { if(argc!=2){set_result(vm,"wait milliseconds expected");return VM_ERROR;}unsigned long ms=strtoul(w[1].text,NULL,10);if(ms>30000){set_result(vm,"wait limit is 30000ms");return VM_ERROR;}/* wait is deliberately capped; it blocks only this script call */
        struct timespec ts; ts.tv_sec=(time_t)(ms/1000);ts.tv_nsec=(long)(ms%1000)*1000000L;
        nanosleep(&ts,NULL);set_result(vm, "");return VM_OK; }
    Mb5Proc *proc=find_proc(vm,cmd);if(proc)return eval_proc(vm,proc,w,argc,depth);
    set_result(vm, "unknown command or procedure");return VM_ERROR;
}

MB5Vm *mb5_vm_new(void) { MB5Vm *vm=(MB5Vm*)calloc(1,sizeof(MB5Vm)); if(vm)set_result(vm,""); return vm; }
void mb5_vm_free(MB5Vm *vm) { if(vm)free(vm); }
int mb5_vm_eval(MB5Vm *vm,const char *source) { if(!vm||!source)return VM_ERROR;return eval_script_depth(vm,source,0); }
int mb5_vm_call(MB5Vm *vm,const char *name,const char *argument) {
    if(!vm||!name)return VM_ERROR;
    char command[MB5_MAX_WORD];
    if(argument&&*argument){
        size_t n=0; const char *p=name;
        while(*p && n+1<sizeof(command)) command[n++]=*p++;
        if(n+2>=sizeof(command)) return VM_ERROR;
        command[n++]=' '; command[n++]='{';
        for(p=argument;*p && n+3<sizeof(command);p++){if(*p=='\\'||*p=='}')command[n++]='\\';command[n++]=*p;}
        command[n++]='}'; command[n]='\0';
    } else { snprintf(command,sizeof(command),"%s",name); }
    return mb5_vm_eval(vm,command);
}
int mb5_vm_has_proc(MB5Vm *vm,const char *name){return vm&&find_proc(vm,name)!=NULL;}
const char *mb5_vm_result(MB5Vm *vm){return vm?vm->result:"invalid VM";}
