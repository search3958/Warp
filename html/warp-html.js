(function() {
    function logSuccess(msg, data = null) {
        console.log(`%c[Warp v3] ✅ ${msg}`, 'color: #2e7d32; font-weight: bold;', data);
    }

    function logError(msg, detail = "") {
        console.error(`%c[Warp v3] ❌ ${msg}`, 'color: #c62828; font-weight: bold;', detail);
    }

    function loadCSS() {
        const cssPath = 'warp-html.css';
        if (!document.querySelector(`link[href="${cssPath}"]`)) {
            const link = document.createElement('link');
            link.rel = 'stylesheet';
            link.href = cssPath;
            document.head.appendChild(link);
        }
    }

    // トークナイザー (V3対応: $や@を考慮)
    function tokenize(code) {
        const tokens = [];
        let pos = 0;
        while (pos < code.length) {
            let c = code[pos];
            if (/\s/.test(c)) { pos++; continue; }
            if (c === '/' && code[pos + 1] === '/') {
                while (pos < code.length && code[pos] !== '\n') pos++;
                continue;
            }
            if (c === '"' || c === "'") {
                let quote = c; let str = quote; pos++;
                while (pos < code.length) {
                    if (code[pos] === '\\') { str += code[pos] + (code[pos+1] || ''); pos += 2; continue; }
                    if (code[pos] === quote) { str += code[pos]; pos++; break; }
                    str += code[pos]; pos++;
                }
                tokens.push(str); continue;
            }
            if (['(', ')', '{', '}', ',', ':', '=', '+', '*', '/', ';', '.', '$', '@'].includes(c)) {
                // シジル (---, --, ~~) の処理
                if (c === '-' && code[pos+1] === '-') {
                    let s = "--"; pos += 2;
                    if (code[pos] === '-') { s = "---"; pos++; }
                    tokens.push(s); continue;
                }
                if (c === '~' && code[pos+1] === '~') {
                    tokens.push("~~"); pos += 2; continue;
                }
                tokens.push(c); pos++; continue;
            }
            let word = "";
            while (pos < code.length && !/\s/.test(code[pos]) && !['(', ')', '{', '}', ',', ':', '=', '+', '*', '/', '"', "'", ';', '.', '$', '@'].includes(code[pos])) {
                word += code[pos]; pos++;
            }
            if (word.length > 0) tokens.push(word);
        }
        return tokens;
    }

    // パーサー (V3対応: Screen, func, @Decorator, Modifier)
    function parseCode(code) {
        const tokens = tokenize(code);
        let pos = 0;

        const peek = (n=0) => tokens[pos + n];
        const next = () => tokens[pos++];

        function parseNode() {
            if (pos >= tokens.length) return null;
            let token = peek();

            // @App / @State / @Storage var name = value
            if (token === '@') {
                let type = next() + next(); // @ + type
                if (peek() === 'var') next();
                let name = next();
                if (peek() === '=') {
                    next();
                    let val = [];
                    while (pos < tokens.length && peek() !== '\n' && peek() !== ',' && peek() !== '}' && peek() !== '{') {
                        val.push(next());
                    }
                    return { type: 'variable', decorator: type, name, value: val };
                }
            }

            // Screen name { ... }
            if (token === 'Screen') {
                next();
                let id = next();
                let node = { type: 'screen', id, children: [] };
                if (peek() === '{') {
                    next();
                    while (pos < tokens.length && peek() !== '}') {
                        let child = parseNode();
                        if (child) node.children.push(child);
                        else pos++;
                    }
                    next();
                }
                return node;
            }

            // func name() { ... }
            if (token === 'func') {
                next();
                let name = next();
                next(); next(); // ()
                let acts = [];
                if (peek() === '{') {
                    next(); let p = 1;
                    while (pos < tokens.length && p > 0) {
                        if (peek() === '{') p++;
                        if (peek() === '}') p--;
                        if (p > 0) acts.push(next()); else next();
                    }
                }
                return { type: 'script', name, actions: acts.join(' ') };
            }

            // Component
            let id = null;
            if (peek(1) === '.') { id = next(); next(); token = peek(); } // id.Comp

            let node = { type: 'component', name: next(), id, props: {}, events: {}, children: [] };
            
            // Label / Params: ("label", text: $var)
            if (peek() === '(') {
                next(); let p = 1;
                while (pos < tokens.length && p > 0) {
                    if (peek() === '(') p++;
                    if (peek() === ')') { p--; if (p === 0) { next(); break; } }
                    
                    let k = peek();
                    if (peek(1) === ':') {
                        next(); next();
                        let v = [];
                        while (pos < tokens.length && peek() !== ',' && peek() !== ')') {
                            v.push(next());
                        }
                        if (['isOn', 'value', 'text'].includes(k)) node.events['output'] = v.join('').replace('$', '');
                        node.props[k] = v;
                    } else {
                        let v = next();
                        if (node.props['text'] === undefined) node.props['text'] = [v];
                    }
                    if (peek() === ',') next();
                }
            }

            // Body
            if (peek() === '{') {
                next();
                while (pos < tokens.length && peek() !== '}') {
                    let saved = pos;
                    let child = parseNode();
                    if (child) {
                        if (child.type === 'variable') {
                            let prefix = child.decorator === '@App' ? '---' : (child.decorator === '@Storage' ? '~~' : '--');
                            node.props[prefix + child.name] = child.value;
                        } else {
                            node.children.push(child);
                        }
                    } else {
                        pos = saved;
                        let act = [];
                        while (pos < tokens.length && peek() !== '\n' && peek() !== ',' && peek() !== '}' && peek() !== '.') {
                            act.push(next());
                        }
                        if (act.length > 0) node.events['_action'] = (node.events['_action'] ? node.events['_action'] + ', ' : '') + act.join(' ');
                    }
                    if (peek() === ',' || peek() === ';') next();
                }
                next();
            }

            // Modifiers: .frame(width: 100)
            while (peek() === '.') {
                next();
                let mname = next();
                if (peek() === '(') {
                    next(); let mv = []; let mp = 1;
                    while (pos < tokens.length && mp > 0) {
                        if (peek() === '(') mp++;
                        if (peek() === ')') { mp--; if (mp === 0) { next(); break; } }
                        mv.push(next());
                    }
                    node.props[mname] = mv;
                } else {
                    node.props[mname] = [true];
                }
            }

            return node;
        }

        const ast = [];
        while (pos < tokens.length) {
            let n = parseNode();
            if (n) ast.push(n);
            else pos++;
        }
        return ast;
    }

    // 表示用評価 (V3対応: \(var) interpolation)
    function evalExpr(exprArr, state) {
        if (!exprArr) return "";
        let raw = exprArr.map(t => t.t === 'nl' ? '\n' : (t.v || t)).join(' ');
        
        // 文字列内の \(var) を置換
        if (raw.startsWith('"') || raw.startsWith("'")) {
            let content = raw.slice(1, -1).replace(/\\n/g, '\n').replace(/\\"/g, '"');
            return content.replace(/\\\(([^)]+)\)/g, (_, k) => {
                let v = resolveState(k.trim(), state);
                return (v !== undefined && v !== null) ? String(v) : "";
            });
        }

        // 変数参照
        if (exprArr.length === 1) {
            let v = resolveState(exprArr[0], state);
            if (v !== undefined) return v;
        }

        return raw;
    }

    function resolveState(name, state) {
        if (state[name] !== undefined) return state[name];
        if (state['--' + name] !== undefined) return state['--' + name];
        if (state['---' + name] !== undefined) return state['---' + name];
        if (state['~~' + name] !== undefined) return state['~~' + name];
        return undefined;
    }

    function setState(name, val, state) {
        if (state['---' + name] !== undefined) state['---' + name] = val;
        else if (state['~~' + name] !== undefined) state['~~' + name] = val;
        else if (state['--' + name] !== undefined) state['--' + name] = val;
        else state['--' + name] = val;
    }

    // 値の評価
    function evaluateValue(valStr, state) {
        valStr = valStr.trim();
        if (valStr.startsWith('(') && valStr.endsWith(')')) valStr = valStr.slice(1, -1).trim();
        if (valStr === 'null') return null;
        if (valStr === 'true') return true;
        if (valStr === 'false') return false;

        let v = resolveState(valStr, state);
        if (v !== undefined) return v;

        if (!isNaN(valStr) && valStr !== "") return parseFloat(valStr);
        if (valStr.startsWith('"') || valStr.startsWith("'")) {
            let content = valStr.slice(1, -1);
            return content.replace(/\\\(([^)]+)\)/g, (_, k) => resolveState(k.trim(), state) ?? "");
        }

        // JS Expression fallback
        try {
            let expr = valStr.replace(/([a-zA-Z_]\w*)/g, m => {
                if (['true', 'false', 'null'].includes(m)) return m;
                let res = resolveState(m, state);
                return (res !== undefined && res !== null) ? (isNaN(res) ? `"${res}"` : Number(res)) : m;
            });
            return Function(`"use strict"; return (${expr})`)();
        } catch(e) { return valStr; }
    }

    function evaluateCondition(cond, state) {
        cond = cond.trim();
        const OPS = ['==', '!=', '>=', '<=', '>', '<'];
        for (let op of OPS) {
            if (cond.includes(op)) {
                let parts = cond.split(op);
                let left = evaluateValue(parts[0], state);
                let right = evaluateValue(parts[1], state);
                switch(op) {
                    case '==': return String(left) === String(right);
                    case '!=': return String(left) !== String(right);
                    case '>=': return parseFloat(left) >= parseFloat(right);
                    case '<=': return parseFloat(left) <= parseFloat(right);
                    case '>': return parseFloat(left) > parseFloat(right);
                    case '<': return parseFloat(left) < parseFloat(right);
                }
            }
        }
        return !!evaluateValue(cond, state);
    }

    function initWarp() {
        loadCSS();
        document.querySelectorAll('warp-code').forEach(block => {
            const container = document.createElement('div'); container.className = 'warp-app-container';
            block.parentNode.insertBefore(container, block.nextSibling); block.style.display = 'none';
            
            const rawState = {};
            const state = new Proxy(rawState, {
                get(t, p) { if (typeof p === 'string' && p.startsWith('~~')) return localStorage.getItem('warp_'+p) || t[p]; return t[p]; },
                set(t, p, v) { t[p] = v; if(typeof p === 'string' && p.startsWith('~~')) localStorage.setItem('warp_'+p, v); return true; }
            });
            
            const ast = parseCode(block.textContent);
            state._currentScreen = 'main'; state._dynamicNodes = {}; state._visibility = {}; state._status = {};

            async function executeAction(actionStr) {
                let acts = []; let cur = ""; let q = false; let p_curly = 0; let p_round = 0;
                for (let i = 0; i < actionStr.length; i++) {
                    let c = actionStr[i]; if (c === '"' || c === "'") q = !q;
                    if (!q) {
                        if (c === '{') p_curly++; if (c === '}') p_curly--;
                        if (c === '(') p_round++; if (c === ')') p_round--;
                    }
                    if (c === ',' && !q && p_curly === 0 && p_round === 0) { acts.push(cur.trim()); cur = ""; } else cur += c;
                }
                if (cur.trim()) acts.push(cur.trim());

                for (let act of acts) {
                    act = act.trim();
                    try {
                        if (act.startsWith('if')) {
                            let p = act.indexOf('{');
                            let cond = act.slice(2, p).trim();
                            if (cond.startsWith('(') && cond.endsWith(')')) cond = cond.slice(1, -1);
                            let aEnd = act.lastIndexOf('}');
                            let inner = act.slice(p + 1, aEnd);
                            if (evaluateCondition(cond, state)) await executeAction(inner);
                        } else if (act.startsWith('wait')) {
                            let m = act.match(/wait[({]([^)}]+)/);
                            await new Promise(r => setTimeout(r, parseFloat(evaluateValue(m[1], state)) * 1000));
                        } else if (act === 'reset()') {
                            location.reload();
                        } else if (act.endsWith('.toggle()')) {
                            let n = act.replace('.toggle()', '').trim();
                            setState(n, !resolveState(n, state), state);
                        } else if (act.startsWith('container(')) {
                            let m = act.match(/container\("([^"]+)"\)\.(add|removeLast|clear)\((.*)\)/);
                            if (m) {
                                let [_, id, method, arg] = m;
                                if (method === 'add') {
                                    if (!state._dynamicNodes[id]) state._dynamicNodes[id] = [];
                                    state._dynamicNodes[id].push(...parseCode(evaluateValue(arg, state)));
                                } else if (method === 'removeLast') {
                                    if (state._dynamicNodes[id]) state._dynamicNodes[id].pop();
                                } else {
                                    state._dynamicNodes[id] = [];
                                }
                            }
                        } else if (act.includes('=')) {
                            let eq = act.indexOf('=');
                            let isPlus = act[eq-1] === '+';
                            let k = act.slice(0, isPlus ? eq-1 : eq).trim();
                            let v = evaluateValue(act.slice(eq+1), state);
                            if (isPlus) v = (resolveState(k, state) || "") + v;
                            setState(k, v, state);
                            if (k === 'currentScreen') state._currentScreen = v;
                        } else {
                            // Function call?
                            let fname = act.replace('()', '').trim();
                            let s = ast.find(n => n.type === 'script' && n.name === fname);
                            if (s) await executeAction(s.actions);
                        }
                    } catch (e) { console.error("Action failed:", act, e); }
                }
                render();
            }

            function applyStyles(el, node) {
                const palette = { yellow: '#fbc02d', red: '#b3261e', blue: '#0a56d0', gray: '#808080', black: '#000000', white: '#ffffff' };
                const format = (v) => {
                    v = String(v);
                    if (/[+\-*/]/.test(v) && !v.includes('calc')) {
                        return `calc(${v.replace(/([0-9.]+)(?![a-zA-Z%])/g, '$1px')})`;
                    }
                    return (isNaN(v) || v === "") ? v : v + "px";
                };

                if (node.props['width']) el.style.width = evalExpr(node.props['width'], state) === 'max' ? '100%' : format(evalExpr(node.props['width'], state));
                if (node.props['height']) el.style.height = format(evalExpr(node.props['height'], state));
                if (node.props['backgroundColor']) el.style.backgroundColor = palette[evalExpr(node.props['backgroundColor'], state)] || evalExpr(node.props['backgroundColor'], state);
                if (node.props['foregroundColor']) el.style.color = palette[evalExpr(node.props['foregroundColor'], state)] || evalExpr(node.props['foregroundColor'], state);
                if (node.props['zIndex']) el.style.zIndex = evalExpr(node.props['zIndex'], state);
                
                ['frame', 'position', 'offset'].forEach(k => {
                    if (node.props[k]) {
                        if (k === 'position') el.style.position = 'fixed';
                        node.props[k].join('').split(',').forEach(s => {
                            let [p, v] = s.split(':').map(x => x.trim()); if (!p || !v) return;
                            if (k === 'offset') el.style['margin'+p.charAt(0).toUpperCase()+p.slice(1)] = format(v);
                            else el.style[p] = format(v);
                        });
                    }
                });
            }

            function renderNode(node) {
                if (!node || node.type === 'script' || node.type === 'variable') return null;
                let el;
                try {
                    let text = evalExpr(node.props['text'], state);
                    switch (node.name) {
                        case 'Header':
                            el = document.createElement('div'); el.className = 'warp-header';
                            let t = document.createElement('h1'); t.innerText = text; el.appendChild(t);
                            let as = document.createElement('div'); as.className = 'warp-header-actions';
                            node.children.forEach(c => { let r = renderNode(c); if(r) as.appendChild(r); });
                            el.appendChild(as); break;
                        case 'Card':
                            el = document.createElement('div'); el.className = 'warp-card';
                            if (text) { let h = document.createElement('h3'); h.innerText = text; el.appendChild(h); }
                            node.children.forEach(c => { let r = renderNode(c); if(r) el.appendChild(r); }); break;
                        case 'Text':
                            el = document.createElement('div'); el.className = 'warp-text'; el.innerText = text; break;
                        case 'Button':
                        case 'TonalButton':
                            el = document.createElement('button'); el.className = 'warp-button' + (node.name === 'TonalButton' ? ' tonal' : '');
                            el.innerText = text;
                            if (node.events['_action']) el.onclick = () => executeAction(node.events['_action']);
                            if (node.props['onLongPress']) {
                                let timer;
                                el.onpointerdown = () => timer = setTimeout(() => executeAction(node.props['onLongPress'].join(' ')), 600);
                                el.onpointerup = () => clearTimeout(timer);
                            }
                            break;
                        case 'HStack': case 'VStack':
                            el = document.createElement('div'); el.className = node.name === 'HStack' ? 'warp-hstack' : 'warp-vstack';
                            node.children.forEach(c => { let r = renderNode(c); if(r) el.appendChild(r); }); break;
                        case 'Toggle':
                            el = document.createElement('input'); el.type = 'checkbox'; el.className = 'warp-switch';
                            el.checked = !!resolveState(node.events['output'], state);
                            el.disabled = !!evalExpr(node.props['disabled'], state);
                            el.onchange = () => { executeAction(`${node.events['output']} = ${el.checked}`); }; break;
                        case 'Slider':
                            el = document.createElement('input'); el.type = 'range'; el.className = 'warp-slider';
                            let rng = String(node.props['in'] || "").split('...');
                            el.min = evaluateValue(rng[0] || "0", state); el.max = evaluateValue(rng[1] || "100", state);
                            el.value = resolveState(node.events['output'], state) || 0;
                            el.oninput = () => { executeAction(`${node.events['output']} = ${el.value}`); }; break;
                        case 'TextField':
                            el = document.createElement('input'); el.className = 'warp-input';
                            el.placeholder = text || "";
                            el.value = resolveState(node.events['output'], state) || "";
                            el.oninput = () => { executeAction(`${node.events['output']} = "${el.value}"`); }; break;
                        default: el = document.createElement('div'); el.innerText = `[${node.name}]`;
                    }
                    if (el) {
                        if (node.id) el.id = node.id;
                        if (node.props['id']) el.id = evalExpr(node.props['id'], state);
                        if (el.id) {
                            if (state._visibility[el.id] === false || evalExpr(node.props['hidden'], state) === true) el.style.display = 'none';
                            if (state._dynamicNodes[el.id]) state._dynamicNodes[el.id].forEach(dn => { let r = renderNode(dn); if(r) el.appendChild(r); });
                        }
                        applyStyles(el, node);
                    }
                    return el;
                } catch (e) { return null; }
            }

            function render() {
                container.innerHTML = '';
                ast.forEach(n => {
                    if (n.type === 'screen' && n.id === state._currentScreen) {
                        n.children.forEach(c => { let r = renderNode(c); if(r) container.appendChild(r); });
                    }
                });
            }

            (function init(ns) {
                ns.forEach(n => {
                    if (n.type === 'variable') {
                        let prefix = n.decorator === '@App' ? '---' : (n.decorator === '@Storage' ? '~~' : '--');
                        state[prefix + n.name] = evaluateValue(n.value.join(' '), state);
                    }
                    if (n.children) init(n.children);
                });
            })(ast);
            render();
            logSuccess("Warp App Ready", ast);
        });
    }

    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', initWarp); else initWarp();
})();
