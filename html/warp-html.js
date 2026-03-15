(function() {
    function logSuccess(msg, data = null) {
        console.log(`%c[Warp Engine] ✅ ${msg}`, 'color: #2e7d32; font-weight: bold;', data);
    }

    function logError(msg, detail = "") {
        console.error(`%c[Warp Engine] ❌ ${msg}`, 'color: #c62828; font-weight: bold;', detail);
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

    // トークナイザー
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
            if (['(', ')', '{', '}', ',', ':', '=', '+', '*', '/', ';'].includes(c)) {
                tokens.push(c); pos++; continue;
            }
            let word = "";
            while (pos < code.length && !/\s/.test(code[pos]) && !['(', ')', '{', '}', ',', ':', '=', '+', '*', '/', '"', "'", ';'].includes(code[pos])) {
                word += code[pos]; pos++;
            }
            if (word.length > 0) tokens.push(word);
        }
        return tokens;
    }

    // パーサー
    function parseCode(code) {
        const tokens = tokenize(code);
        let pos = 0;
        const isReserved = (n) => ['reset', 'calc', 'script', 'add', 'del', 'clr', 'show', 'hide', 'wait', 'setScreen'].includes(n);

        function parseNode() {
            if (pos >= tokens.length) return null;
            let token = tokens[pos];

            if (token.startsWith('@')) {
                let node = { type: 'script', name: token.slice(1), blocks: [] };
                pos += 2; 
                while (pos < tokens.length && tokens[pos] !== '}') {
                    let type = tokens[pos++]; if (tokens[pos] === ':') pos++;
                    if (tokens[pos] === '(') pos++;
                    let cond = []; while (pos < tokens.length && tokens[pos] !== ')') cond.push(tokens[pos++]);
                    if (tokens[pos] === ')') pos++;
                    if (tokens[pos] === '{') pos++;
                    let acts = []; let p = 1;
                    while (pos < tokens.length && p > 0) {
                        if (tokens[pos] === '{') p++; if (tokens[pos] === '}') p--;
                        if (p > 0) acts.push(tokens[pos++]); else pos++;
                    }
                    node.blocks.push({ type, condition: cond.join(''), actions: acts.join('') });
                    if (tokens[pos] === ',') pos++;
                }
                pos++; return node;
            }

            if (pos + 1 < tokens.length && tokens[pos + 1] === '{') {
                let node = { type: 'component', name: token, props: {}, events: {}, children: [] };
                pos += 2;
                while (pos < tokens.length && tokens[pos] !== '}') {
                    let t = tokens[pos];
                    if (pos + 1 < tokens.length && tokens[pos+1] === ':') {
                        let key = t; pos += 2;
                        if (tokens[pos] !== '(') { pos++; continue; }
                        pos++; let val = []; let p = 1;
                        while (pos < tokens.length && p > 0) {
                            if (tokens[pos] === '(') p++; if (tokens[pos] === ')') p--;
                            if (p > 0) val.push(tokens[pos++]); else pos++;
                        }
                        if (['oneClick', 'longPress', 'output'].includes(key)) node.events[key] = val.join('');
                        else node.props[key] = val;
                        if (tokens[pos] === ',') pos++; continue;
                    }
                    if (pos + 1 < tokens.length && tokens[pos+1] === '{') {
                        let child = parseNode(); if (child) node.children.push(child);
                        if (tokens[pos] === ',') pos++; continue;
                    }
                    pos++;
                }
                pos++; return node;
            }
            return null;
        }
        const ast = []; while (pos < tokens.length) { let n = parseNode(); if (n) ast.push(n); else pos++; }
        return ast;
    }

    // 表示用評価
    function evalExpr(exprArr, state) {
        if (!exprArr) return "";
        let res = "";
        for (let t of exprArr) {
            if (t === '+') continue;
            if (t.startsWith('--') || t.startsWith('~~')) {
                let v = state[t];
                res += (v !== undefined && v !== null) ? String(v) : "";
            } else if (t.startsWith('"') || t.startsWith("'")) {
                res += t.slice(1, -1).replace(/\\n/g, '\n').replace(/\\"/g, '"');
            } else if (t === 'null') {
                res += "";
            } else {
                res += t;
            }
        }
        return res;
    }

    // 値の評価 (代入・条件用)
    function evaluateValue(valStr, state) {
        valStr = valStr.trim();
        if (valStr.startsWith('(') && valStr.endsWith(')')) valStr = valStr.slice(1, -1).trim();
        if (valStr === 'null') return null;
        if (valStr === 'true') return true;
        if (valStr === 'false') return false;

        // .replace{}
        if (valStr.includes('.replace{')) {
            let parts = valStr.split('.replace{');
            let vn = parts[0].trim();
            let argsStr = parts[1].split('}')[0];
            let args = argsStr.split(',').map(a => evaluateValue(a.trim(), state));
            let base = state[vn] !== undefined ? String(state[vn]) : evaluateValue(vn, state);
            return String(base || "").replace(args[0], args[1] || "");
        }

        // calc{}
        if (valStr.startsWith('calc{') && valStr.endsWith('}')) {
            let inner = valStr.slice(5, -1).replace(/(--[a-zA-Z0-9_-]+|~~[a-zA-Z0-9_-]+)/g, m => state[m] || "0");
            try { return String(eval(inner.replace(/[^0-9+\-*/().\s]/g, ''))); } catch(e) { return "Error"; }
        }
        
        // 文字列結合
        if (valStr.includes('+')) {
            let parts = []; let cur = ""; let q = false;
            for(let i=0; i<valStr.length; i++){
                if(valStr[i] === '"' || valStr[i] === "'") q = !q;
                if(valStr[i] === '+' && !q) { parts.push(cur.trim()); cur = ""; } else cur += valStr[i];
            }
            parts.push(cur.trim());
            return parts.map(p => {
                let pv = p.trim();
                if (pv.startsWith('"') || pv.startsWith("'")) return pv.slice(1, -1);
                if (pv.startsWith('--') || pv.startsWith('~~')) return state[pv] !== undefined && state[pv] !== null ? state[pv] : "";
                return pv;
            }).join('');
        }

        if (!isNaN(valStr) && valStr !== "") return parseFloat(valStr);
        if (valStr.startsWith('"') || valStr.startsWith("'")) return valStr.slice(1, -1);
        return state[valStr] !== undefined ? state[valStr] : valStr;
    }

    function evaluateCondition(cond, state) {
        cond = cond.trim();
        if (cond.includes('.contains{')) {
            let parts = cond.split('.contains{');
            let vn = parts[0].trim();
            let search = evaluateValue(parts[1].split('}')[0], state);
            let has = String(state[vn] || "").includes(search);
            return cond.includes('= false') ? !has : has;
        }
        if (cond.includes('=')) {
            let parts = cond.split('=');
            let left = evaluateValue(parts[0], state);
            let right = evaluateValue(parts.slice(1).join('='), state);
            return String(left) === String(right);
        }
        return !!evaluateValue(cond, state);
    }

    function initWarp() {
        loadCSS();
        document.querySelectorAll('warp-code').forEach(block => {
            const container = document.createElement('div'); container.className = 'warp-app-container';
            block.parentNode.insertBefore(container, block.nextSibling); block.style.display = 'none';
            const state = new Proxy({}, {
                get(t, p) { return p.startsWith('~~') ? localStorage.getItem('warp_'+p) || t[p] : t[p]; },
                set(t, p, v) { t[p] = v; if(p.startsWith('~~')) localStorage.setItem('warp_'+p, v); return true; }
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
                        if (act.startsWith('if:')) {
                            let p = act.indexOf('(');
                            let depth = 0; let condEnd = -1;
                            for (let i = p; i < act.length; i++) {
                                if (act[i] === '(') depth++;
                                if (act[i] === ')') { depth--; if (depth === 0) { condEnd = i; break; } }
                            }
                            let cond = act.slice(p + 1, condEnd).trim();
                            let aStart = act.indexOf('{', condEnd);
                            let aDepth = 0; let aEnd = -1;
                            for (let i = aStart; i < act.length; i++) {
                                if (act[i] === '{') aDepth++;
                                if (act[i] === '}') { aDepth--; if (aDepth === 0) { aEnd = i; break; } }
                            }
                            let inner = act.slice(aStart + 1, aEnd);
                            if (evaluateCondition(cond, state)) await executeAction(inner);
                        } else if (act.startsWith('wait:')) {
                            let val = act.split('wait:')[1].trim();
                            await new Promise(r => setTimeout(r, parseFloat(evaluateValue(val, state)) * 1000));
                        } else if (act.startsWith('setScreen{')) {
                            state._currentScreen = act.slice(10, -1).trim();
                        } else if (act.startsWith('script{')) {
                            await executeScript(act.slice(7, -1).trim());
                        } else if (act.startsWith('show{')) {
                            state._visibility[act.slice(5, -1).trim()] = true;
                        } else if (act.startsWith('hide{')) {
                            state._visibility[act.slice(5, -1).trim()] = false;
                        } else if (act.startsWith('clr{')) {
                            state._dynamicNodes[act.slice(4, -1).trim()] = [];
                        } else if (act.startsWith('add{')) {
                            let idx = act.indexOf(':'); let target = act.slice(4, idx).trim();
                            let comp = evaluateValue(act.slice(idx+1, -1).trim(), state);
                            if (!state._dynamicNodes[target]) state._dynamicNodes[target] = [];
                            state._dynamicNodes[target].push(...parseCode(comp));
                        } else if (act.startsWith('del{')) {
                            let target = act.slice(4, -1).split(':')[0].trim();
                            if (state._dynamicNodes[target]) state._dynamicNodes[target].pop();
                        } else if (act.startsWith('reset{')) {
                            location.reload();
                        } else if (act.includes('.setStatus{')) {
                            let [id, val] = act.split('.setStatus{'); state._status[id.trim()] = val.slice(0, -1).trim();
                        } else if (act.includes('.changeContent{')) {
                            let id = act.split('.changeContent{')[0].trim();
                            let val = evaluateValue(act.split('.changeContent{')[1].slice(0, -1).trim(), state);
                            let el = document.getElementById(id); if (el) { el.value = val; el.dispatchEvent(new Event('input')); }
                        } else if (act.includes('=')) {
                            let parts = act.split('=');
                            let k = parts[0].trim();
                            state[k] = evaluateValue(parts.slice(1).join('='), state);
                        }
                    } catch (e) { logError(`Action failed: ${act}`, e); }
                }
                render();
            }

            async function executeScript(name) {
                let s = ast.find(n => n.type === 'script' && n.name === name);
                if (!s) return; let done = false;
                for (let b of s.blocks) {
                    let match = evaluateCondition(b.condition, state);
                    if (match && (!done || b.type === 'if')) { await executeAction(b.actions); done = true; }
                }
            }

            function applyStyles(el, node) {
                const palette = { yellow: '#fbc02d', red: '#b3261e', blue: '#0a56d0', gray: '#808080', black: '#000000', white: '#ffffff' };
                const format = (v) => {
                    if (isNaN(v) && /[+\-*/]/.test(v) && !v.includes('calc')) {
                        let spaced = v.replace(/([+\-*/])/g, ' $1 ').replace(/\s+/g, ' ').trim();
                        let withPx = spaced.replace(/\b([0-9.]+)\b(?!\s*[a-zA-Z%])/g, '$1px');
                        return `calc(${withPx})`;
                    }
                    return (isNaN(v) || v === "") ? v : v + "px";
                };

                if (node.props['width']) {
                    let w = evalExpr(node.props['width'], state);
                    el.style.width = w === 'max' ? '100%' : format(w);
                }
                if (node.props['height']) {
                    let h = evalExpr(node.props['height'], state);
                    el.style.height = h === 'max' ? '100%' : format(h);
                }

                ['frame', 'position', 'offset'].forEach(k => {
                    if (node.props[k]) {
                        if (k === 'position') el.style.position = 'fixed';
                        node.props[k].join('').split(',').forEach(s => {
                            let [p, v] = s.split('=').map(x => x.trim()); if (!p || !v) return;
                            if (k === 'offset') el.style['margin'+p.charAt(0).toUpperCase()+p.slice(1)] = format(v);
                            else el.style[p] = format(v);
                        });
                    }
                });
                if (node.props['zIndex']) el.style.zIndex = node.props['zIndex'].join('');
                let bgColor = evalExpr(node.props['color'], state);
                if (bgColor) el.style.backgroundColor = palette[bgColor] || bgColor;
                let textColor = evalExpr(node.props['textColor'], state);
                if (textColor) el.style.color = palette[textColor] || textColor;
            }

            function renderNode(node) {
                if (!node || node.type === 'script') return null;
                let el;
                try {
                    switch (node.name) {
                        case 'Header':
                            el = document.createElement('div'); el.className = 'warp-header';
                            let t = document.createElement('h1'); t.innerText = evalExpr(node.props['text'], state); el.appendChild(t);
                            let as = document.createElement('div'); as.className = 'warp-header-actions';
                            node.children.forEach(c => { let r = renderNode(c); if(r) as.appendChild(r); });
                            el.appendChild(as); break;
                        case 'card':
                            el = document.createElement('div'); el.className = 'warp-card';
                            if (node.props['text']) { let h = document.createElement('h3'); h.innerText = evalExpr(node.props['text'], state); el.appendChild(h); }
                            node.children.forEach(c => { let r = renderNode(c); if(r) el.appendChild(r); }); break;
                        case 'text':
                            el = document.createElement('div'); el.className = 'warp-text'; el.innerText = evalExpr(node.props['text'], state); break;
                        case 'button':
                        case 'tonalButton':
                            el = document.createElement('button'); el.className = 'warp-button' + (node.name === 'tonalButton' ? ' tonal' : '');
                            el.innerText = evalExpr(node.props['text'], state);
                            if (node.events['oneClick']) el.onclick = () => executeAction(node.events['oneClick']);
                            break;
                        case 'hStack': case 'vStack':
                            el = document.createElement('div'); el.className = node.name === 'hStack' ? 'warp-hstack' : 'warp-vstack';
                            node.children.forEach(c => { let r = renderNode(c); if(r) el.appendChild(r); }); break;
                        case 'switch':
                            el = document.createElement('input'); el.type = 'checkbox'; el.className = 'warp-switch';
                            let sVal = String(state[node.events['output']] || evalExpr(node.props['status'], state));
                            el.checked = sVal.includes('true'); el.disabled = sVal.includes('Disabled');
                            el.onchange = () => { state[node.events['output']] = el.checked ? "true" : "false"; render(); }; break;
                        case 'slider':
                            el = document.createElement('input'); el.type = 'range'; el.className = 'warp-slider';
                            el.max = evalExpr(node.props['max'], state) || 100;
                            el.value = state[node.events['output']] || evalExpr(node.props['status'], state) || 0;
                            el.oninput = () => { state[node.events['output']] = el.value; render(); }; break;
                        case 'input':
                            el = document.createElement('input'); el.className = 'warp-input';
                            el.placeholder = evalExpr(node.props['placeholder'], state) || "";
                            el.value = state[node.events['output']] || evalExpr(node.props['--inputMain'], state) || "";
                            el.oninput = () => { state[node.events['output']] = el.value; render(); }; break;
                        default: el = document.createElement('div'); el.innerText = `[${node.name}]`;
                    }
                    if (el) {
                        let id = evalExpr(node.props['id'], state);
                        if (id) {
                            el.id = id;
                            let status = state._status[id] || evalExpr(node.props['status'], state);
                            if (status === 'disabled' || status === 'falseDisabled') el.disabled = true;
                            else if (status === 'unset') el.disabled = false;
                            if (state._visibility[id] === false) el.style.display = 'none';
                            if (state._dynamicNodes[id]) state._dynamicNodes[id].forEach(dn => { let r = renderNode(dn); if(r) el.appendChild(r); });
                        }
                        applyStyles(el, node);
                    }
                    return el;
                } catch (e) { return null; }
            }

            function render() {
                container.innerHTML = '';
                ast.forEach(n => {
                    if (n.name === 'screen' && evalExpr(n.props['id'], state) === state._currentScreen) {
                        n.children.forEach(c => { let r = renderNode(c); if(r) container.appendChild(r); });
                    }
                });
            }

            (function init(ns) {
                ns.forEach(n => {
                    if (n.type === 'component') {
                        for(let k in n.props) if (k.startsWith('--')) state[k] = evaluateValue(n.props[k].join(''), state);
                        init(n.children);
                    }
                });
            })(ast);
            render();
            logSuccess("Warp App Initialized", ast);
        });
    }

    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', initWarp); else initWarp();
})();
