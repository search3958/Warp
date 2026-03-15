(function() {
    // CSSの自動読み込み
    function loadCSS() {
        const cssPath = 'warp-html.css';
        if (!document.querySelector(`link[href="${cssPath}"]`)) {
            const link = document.createElement('link');
            link.rel = 'stylesheet';
            link.href = cssPath;
            document.head.appendChild(link);
        }
    }

    // トークナイザー: DSLを最小単位の単語に分解
    function tokenize(code) {
        const tokens = [];
        let pos = 0;
        while (pos < code.length) {
            let c = code[pos];
            if (/\s/.test(c)) { pos++; continue; }
            
            // 文字列リテラル
            if (c === '"' || c === "'") {
                let quote = c;
                let str = quote;
                pos++;
                while (pos < code.length) {
                    if (code[pos] === '\\') {
                        str += code[pos] + (code[pos+1] || '');
                        pos += 2; continue;
                    }
                    if (code[pos] === quote) {
                        str += code[pos]; pos++; break;
                    }
                    str += code[pos]; pos++;
                }
                tokens.push(str);
                continue;
            }
            
            // 記号
            if (['(', ')', ',', ':', '=', '+', '.', '{', '}', ';'].includes(c)) {
                tokens.push(c);
                pos++;
                continue;
            }
            
            // 単語（プロパティ、変数、コンポーネント名など）
            let word = "";
            while (pos < code.length && !/\s/.test(code[pos]) && !['(', ')', ',', ':', '=', '+', '"', "'", '.', '{', '}', ';'].includes(code[pos])) {
                word += code[pos];
                pos++;
            }
            if (word.length > 0) tokens.push(word);
        }
        return tokens;
    }

    // パーサー: トークン列から抽象構文木(AST)を構築
    function parseCode(code) {
        const tokens = tokenize(code);
        let pos = 0;

        function isReservedFunc(name) {
            const reserved = ['reset', 'calc', 'script', 'add', 'del', 'clr', 'show', 'hide', 'wait'];
            return reserved.includes(name) || name.startsWith('setScreen');
        }

        function parseNode() {
            if (pos >= tokens.length) return null;
            let token = tokens[pos];

            // スクリプト定義 (@name)
            if (token.startsWith('@')) {
                let name = token.slice(1);
                let node = { type: 'script', name: name, blocks: [] };
                pos += 2; // @name (
                while (pos < tokens.length && tokens[pos] !== ')') {
                    let blockType = tokens[pos++]; // if or elseIf
                    if (tokens[pos] === ':') pos++;
                    
                    let condTokens = [];
                    while (pos < tokens.length && tokens[pos] !== '(') {
                        condTokens.push(tokens[pos++]);
                    }
                    if (tokens[pos] === '(') pos++;
                    
                    let actionTokens = [];
                    let parenCount = 1;
                    while (pos < tokens.length && parenCount > 0) {
                        if (tokens[pos] === '(') parenCount++;
                        if (tokens[pos] === ')') parenCount--;
                        if (parenCount > 0) actionTokens.push(tokens[pos]);
                        pos++;
                    }
                    node.blocks.push({ 
                        type: blockType, 
                        condition: condTokens.join(''), 
                        actions: actionTokens.join('') 
                    });
                    if (tokens[pos] === ',') pos++;
                }
                pos++; // )
                return node;
            }

            // コンポーネント (name(...))
            if (pos + 1 < tokens.length && tokens[pos + 1] === '(') {
                let node = {
                    type: 'component',
                    name: token,
                    props: {},
                    events: {},
                    children: []
                };
                pos += 2;
                while (pos < tokens.length && tokens[pos] !== ')') {
                    let t = tokens[pos];
                    
                    // 子コンポーネント
                    if (pos + 1 < tokens.length && tokens[pos + 1] === '(' && !isReservedFunc(t) && !/^[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+$/.test(t)) {
                        let child = parseNode();
                        if (child) node.children.push(child);
                        if (tokens[pos] === ',') pos++;
                        continue;
                    }
                    
                    // プロパティ (key: value)
                    if (pos + 1 < tokens.length && tokens[pos + 1] === ':') {
                        let key = t;
                        pos += 2;
                        let expr = [];
                        let parenCount = 0;
                        while (pos < tokens.length) {
                            let curr = tokens[pos];
                            if (parenCount === 0) {
                                if (curr === ')' || curr === ',') break;
                                if (pos + 1 < tokens.length && tokens[pos+1] === ':' && !isReservedFunc(curr)) break;
                                if (pos + 1 < tokens.length && tokens[pos+1] === '(' && !isReservedFunc(curr) && !curr.includes('.')) break;
                            }
                            if (curr === '(') parenCount++;
                            if (curr === ')') parenCount--;
                            expr.push(curr);
                            pos++;
                        }
                        
                        if (['oneClick', 'longPress', 'output'].includes(key)) {
                            node.events[key] = expr.join('');
                        } else {
                            node.props[key] = expr;
                        }
                        if (tokens[pos] === ',') pos++;
                        continue;
                    }
                    pos++;
                }
                pos++;
                return node;
            }
            pos++;
            return null;
        }

        const ast = [];
        while (pos < tokens.length) {
            let node = parseNode();
            if (node) ast.push(node);
        }
        return ast;
    }

    // 式の評価 (文字列結合、変数展開)
    function evalExpr(exprArr, state) {
        if (!exprArr || exprArr.length === 0) return "";
        let result = "";
        for (let i = 0; i < exprArr.length; i++) {
            let t = exprArr[i];
            if (t === '+') continue;
            if (t.startsWith('--') || t.startsWith('~~')) {
                let val = state[t];
                result += (val !== undefined && val !== null) ? String(val) : "";
            } else if ((t.startsWith('"') && t.endsWith('"')) || (t.startsWith("'") && t.endsWith("'"))) {
                let inner = t.slice(1, -1);
                // エスケープ解除
                inner = inner.replace(/\\n/g, '\n').replace(/\\"/g, '"').replace(/\\'/g, "'").replace(/\\\\/g, "\\");
                result += inner;
            } else {
                result += t;
            }
        }
        return result;
    }

    // 右辺の評価 (calc() や複雑な代入値)
    function evaluateValue(valStr, state) {
        valStr = valStr.trim();
        if (valStr.startsWith('calc(') && valStr.endsWith(')')) {
            let inner = valStr.slice(5, -1);
            inner = inner.replace(/(--[a-zA-Z0-9_-]+|~~[a-zA-Z0-9_-]+)/g, match => {
                let v = state[match];
                return (v !== undefined && v !== null) ? String(v) : "0";
            });
            try {
                return String(eval(inner.replace(/[^0-9+\-*/().\s]/g, '')));
            } catch(e) { return "Error"; }
        }
        
        // 文字列結合の簡易処理
        if (valStr.includes('+')) {
            let parts = [];
            let current = "";
            let inQuote = false;
            for(let i=0; i<valStr.length; i++) {
                if (valStr[i] === '"' || valStr[i] === "'") inQuote = !inQuote;
                if (valStr[i] === '+' && !inQuote) {
                    parts.push(current.trim());
                    current = "";
                } else {
                    current += valStr[i];
                }
            }
            parts.push(current.trim());
            return parts.map(p => {
                if ((p.startsWith('"') && p.endsWith('"')) || (p.startsWith("'") && p.endsWith("'"))) return p.slice(1, -1);
                if (p.startsWith('--') || p.startsWith('~~')) return state[p] || "";
                return p;
            }).join('');
        }

        if ((valStr.startsWith('"') && valStr.endsWith('"')) || (valStr.startsWith("'") && valStr.endsWith("'"))) {
            return valStr.slice(1, -1);
        }
        if (valStr === 'null') return null;
        if (valStr === 'true') return true;
        if (valStr === 'false') return false;
        if (!isNaN(valStr)) return parseFloat(valStr);
        
        // 変数そのまま
        if (valStr.startsWith('--') || valStr.startsWith('~~')) return state[valStr];

        return valStr;
    }

    function evaluateCondition(condStr, state) {
        try {
            // 現在は単純な '=' 比較のみサポート
            if (condStr.includes('=')) {
                let parts = condStr.split('=');
                let left = evaluateValue(parts[0], state);
                let right = evaluateValue(parts[1], state);
                return String(left) === String(right);
            }
            return !!evaluateValue(condStr, state);
        } catch (e) { return false; }
    }

    function initWarp() {
        loadCSS();
        document.querySelectorAll('warp-code').forEach(block => {
            const code = block.textContent;
            const container = document.createElement('div');
            container.className = 'warp-app-container';
            block.parentNode.insertBefore(container, block.nextSibling);
            block.style.display = 'none';

            // 状態管理
            const state = new Proxy({}, {
                get(target, prop) {
                    if (typeof prop === 'string' && prop.startsWith('~~')) {
                        return localStorage.getItem('warp_' + prop) || target[prop];
                    }
                    return target[prop];
                },
                set(target, prop, value) {
                    target[prop] = value;
                    if (typeof prop === 'string' && prop.startsWith('~~')) {
                        localStorage.setItem('warp_' + prop, value);
                    }
                    return true;
                }
            });

            const ast = parseCode(code);
            state._currentScreen = 'main';
            state._dynamicNodes = {};
            state._visibility = {};
            state._status = {}; // Component ID -> Status (disabled, etc)

            // アクション実行エンジン
            async function executeAction(actionStr) {
                let actions = [];
                let current = "";
                let inQuote = false;
                let paren = 0;
                for (let i = 0; i < actionStr.length; i++) {
                    let c = actionStr[i];
                    if (c === '"' || c === "'") inQuote = !inQuote;
                    if (!inQuote) {
                        if (c === '(') paren++;
                        if (c === ')') paren--;
                    }
                    if (c === ',' && !inQuote && paren === 0) {
                        actions.push(current.trim());
                        current = "";
                    } else {
                        current += c;
                    }
                }
                if (current.trim()) actions.push(current.trim());

                for (let act of actions) {
                    try {
                        if (act.startsWith('wait:')) {
                            let sec = parseFloat(act.split(':')[1]);
                            await new Promise(r => setTimeout(r, sec * 1000));
                        } else if (act.startsWith('setScreen(')) {
                            state._currentScreen = act.slice(10, -1).trim();
                        } else if (act.startsWith('script(')) {
                            await executeScript(act.slice(7, -1).trim());
                        } else if (act.startsWith('show(')) {
                            state._visibility[act.slice(5, -1).trim()] = true;
                        } else if (act.startsWith('hide(')) {
                            state._visibility[act.slice(5, -1).trim()] = false;
                        } else if (act.startsWith('clr(')) {
                            state._dynamicNodes[act.slice(4, -1).trim()] = [];
                        } else if (act.startsWith('add(')) {
                            let inner = act.slice(4, -1);
                            let idx = inner.indexOf(':');
                            let target = inner.slice(0, idx).trim();
                            let comp = inner.slice(idx + 1).trim();
                            if (comp.startsWith("'") || comp.startsWith('"')) comp = comp.slice(1, -1);
                            if (!state._dynamicNodes[target]) state._dynamicNodes[target] = [];
                            state._dynamicNodes[target].push(...parseCode(comp));
                        } else if (act.startsWith('del(')) {
                            let inner = act.slice(4, -1);
                            let idx = inner.indexOf(':');
                            let target = idx > -1 ? inner.slice(0, idx).trim() : inner.trim();
                            if (state._dynamicNodes[target]) state._dynamicNodes[target].pop();
                        } else if (act.startsWith('reset(')) {
                            location.reload();
                        } else if (act.includes('.setStatus(')) {
                            let [id, rest] = act.split('.setStatus(');
                            state._status[id] = rest.slice(0, -1).trim();
                        } else if (act.includes('.changeContent(')) {
                            let [id, rest] = act.split('.changeContent(');
                            let val = evaluateValue(rest.slice(0, -1).trim(), state);
                            let el = document.getElementById(id);
                            if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA')) {
                                el.value = val;
                                // outputイベントがあれば発火
                                el.dispatchEvent(new Event('input'));
                            }
                        } else if (act.includes('=')) {
                            let parts = act.split('=');
                            let key = parts[0].trim();
                            let val = evaluateValue(parts.slice(1).join('='), state);
                            
                            // 文字列メソッドの擬似サポート
                            if (key.includes('.contains(')) { /* Read-only check in if */ }
                            else if (key.includes('.replace(')) {
                                let [varName, method] = key.split('.replace(');
                                let args = method.slice(0, -1).split(',');
                                let oldV = evaluateValue(args[0], state);
                                let newV = evaluateValue(args[1], state) || "";
                                state[varName] = String(state[varName]).replace(oldV, newV);
                            } else {
                                state[key] = val;
                            }
                        }
                    } catch (e) { console.error("Action error:", act, e); }
                }
                render();
            }

            async function executeScript(name) {
                let script = ast.find(n => n.type === 'script' && n.name === name);
                if (!script) return;
                let done = false;
                for (let block of script.blocks) {
                    let cond = block.condition;
                    // contains などの特殊判定
                    let match = false;
                    if (cond.includes('.contains(')) {
                        let [varName, rest] = cond.split('.contains(');
                        let search = evaluateValue(rest.split(')')[0], state);
                        let invert = cond.includes('= false');
                        let has = String(state[varName.trim()]).includes(search);
                        match = invert ? !has : has;
                    } else {
                        match = evaluateCondition(cond, state);
                    }

                    if (block.type === 'if' && match) {
                        await executeAction(block.actions);
                        done = true;
                    } else if (block.type === 'elseIf' && !done && match) {
                        await executeAction(block.actions);
                        done = true;
                    }
                }
            }

            function applyStyles(el, node) {
                // frame: width = 100vw - 40, height = 40
                if (node.props['frame']) {
                    let frame = node.props['frame'].join('');
                    frame.split(',').forEach(s => {
                        let [k, v] = s.split('=');
                        if (k && v) el.style[k.trim()] = v.trim();
                    });
                }
                // position: top = 20, left = 20
                if (node.props['position']) {
                    el.style.position = 'absolute';
                    let pos = node.props['position'].join('');
                    pos.split(',').forEach(s => {
                        let [k, v] = s.split('=');
                        if (k && v) el.style[k.trim()] = v.trim();
                    });
                }
                // offset: left = 10
                if (node.props['offset']) {
                    let off = node.props['offset'].join('');
                    off.split(',').forEach(s => {
                        let [k, v] = s.split('=');
                        if (k && v) el.style['margin' + k.trim().charAt(0).toUpperCase() + k.trim().slice(1)] = v.trim() + 'px';
                    });
                }
                if (node.props['zIndex']) el.style.zIndex = node.props['zIndex'].join('');
                
                // color
                let color = evalExpr(node.props['color'], state);
                if (color) {
                    const palette = { yellow: '#fbc02d', red: '#b3261e', blue: '#0a56d0', gray: '#808080', black: '#000000', unset: '' };
                    let hex = palette[color] || color;
                    if (node.name === 'tonalButton') {
                        el.style.color = hex;
                        el.style.backgroundColor = hex + '13';
                    } else {
                        el.style.color = hex;
                    }
                }
            }

            function renderNode(node) {
                if (node.type === 'script') return null;
                let el;
                switch (node.name) {
                    case 'Header':
                        el = document.createElement('div');
                        el.className = 'warp-header';
                        let t = document.createElement('h1');
                        t.innerText = evalExpr(node.props['text'], state);
                        el.appendChild(t);
                        let acts = document.createElement('div');
                        acts.className = 'warp-header-actions';
                        node.children.forEach(c => { let r = renderNode(c); if(r) acts.appendChild(r); });
                        el.appendChild(acts);
                        break;
                    case 'card':
                        el = document.createElement('div');
                        el.className = 'warp-card';
                        if (node.props['text']) {
                            let h = document.createElement('h3');
                            h.innerText = evalExpr(node.props['text'], state);
                            el.appendChild(h);
                        }
                        node.children.forEach(c => { let r = renderNode(c); if(r) el.appendChild(r); });
                        break;
                    case 'text':
                        el = document.createElement('div');
                        el.className = 'warp-text';
                        el.innerText = evalExpr(node.props['text'], state);
                        break;
                    case 'button':
                    case 'tonalButton':
                        el = document.createElement('button');
                        el.className = 'warp-button' + (node.name === 'tonalButton' ? ' tonal' : '');
                        el.innerText = evalExpr(node.props['text'], state);
                        if (node.events['oneClick']) el.onclick = () => executeAction(node.events['oneClick']);
                        if (node.events['longPress']) {
                            let timer;
                            el.onmousedown = () => timer = setTimeout(() => executeAction(node.events['longPress']), 600);
                            el.onmouseup = el.onmouseleave = () => clearTimeout(timer);
                        }
                        break;
                    case 'hStack':
                    case 'vStack':
                        el = document.createElement('div');
                        el.className = node.name === 'hStack' ? 'warp-hstack' : 'warp-vstack';
                        node.children.forEach(c => { let r = renderNode(c); if(r) el.appendChild(r); });
                        break;
                    case 'switch':
                        el = document.createElement('div');
                        el.className = 'warp-switch-container';
                        let input = document.createElement('input');
                        input.type = 'checkbox';
                        let status = String(state[node.events['output']] || evalExpr(node.props['status'], state));
                        input.checked = status.includes('true');
                        input.disabled = status.includes('Disabled');
                        input.onchange = () => {
                            let val = input.checked ? "true" : "false";
                            if (node.events['output']) {
                                state[node.events['output']] = val;
                                render();
                            }
                        };
                        el.appendChild(input);
                        break;
                    case 'slider':
                        el = document.createElement('input');
                        el.type = 'range';
                        el.className = 'warp-slider';
                        el.max = evalExpr(node.props['max'], state) || 100;
                        el.step = evalExpr(node.props['step'], state) || 1;
                        el.value = state[node.events['output']] || evalExpr(node.props['status'], state) || 0;
                        el.oninput = () => {
                            if (node.events['output']) {
                                state[node.events['output']] = el.value;
                                render();
                            }
                        };
                        break;
                    case 'input':
                        el = document.createElement('input');
                        el.className = 'warp-input';
                        el.placeholder = evalExpr(node.props['placeholder'], state) || "";
                        el.value = state[node.events['output']] || evalExpr(node.props['--inputMain'], state) || "";
                        el.oninput = () => {
                            if (node.events['output']) {
                                state[node.events['output']] = el.value;
                                render();
                            }
                        };
                        break;
                    default:
                        el = document.createElement('div');
                        el.innerText = `[${node.name}]`;
                }

                if (el) {
                    let id = evalExpr(node.props['id'], state);
                    if (id) {
                        el.id = id;
                        // ステータス反映
                        if (state._status[id] === 'disabled') el.disabled = true;
                        if (state._status[id] === 'unset') el.disabled = false;
                        // 可視性反映
                        if (state._visibility[id] === false) el.style.display = 'none';
                        // 動的ノード追加
                        if (state._dynamicNodes[id]) {
                            state._dynamicNodes[id].forEach(dn => {
                                let r = renderNode(dn);
                                if(r) el.appendChild(r);
                            });
                        }
                    }
                    applyStyles(el, node);
                }
                return el;
            }

            function render() {
                container.innerHTML = '';
                ast.forEach(node => {
                    if (node.name === 'screen') {
                        let id = evalExpr(node.props['id'], state);
                        if (id === state._currentScreen) {
                            node.children.forEach(c => {
                                let r = renderNode(c);
                                if(r) container.appendChild(r);
                            });
                        }
                    }
                });
            }

            // 初期状態の構築
            function initState(nodes) {
                nodes.forEach(n => {
                    if (n.type === 'component') {
                        for(let k in n.props) {
                            if (k.startsWith('--')) state[k] = evalExpr(n.props[k], state);
                        }
                        initState(n.children);
                    }
                });
            }
            initState(ast);
            render();
        });
    }

    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', initWarp);
    else initWarp();
})();
