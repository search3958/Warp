package com.example.warp

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.warp.ui.theme.WarpTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        
        // Sentaroさんが作成したWarpコード
        val sampleWarpCode = """
screen(
    id:main

    Header(
    text:"Warp Demo"

    button(
        --headerText:""
        text:"⚠️"+--headerText
        oneClick:--headerText="Please wait",reset(now)
    )
    )

    text(
    text:"これは新しい、ネイティブとWeb技術が融合する言語です。気になる反応は"+--mainText
    --mainText:"..."
    )

    card(
    text:"内容変更"
    text(
        text:"クリックにより表示内容が変更されます"
    )
    hStack(
        button(
            text:"👍"
            oneClick:--mainText="良いみたいです!"
        )
        button(
            text:"🤔"
            oneClick:--mainText="あまり良くないようですね。"
        )
    )
        button(
            text:"Click"
            --color:yellow
            color:--color
            oneClick:--color=red
            longPress:--color=blue
        )
    )

    card(
        text:"button"
        text(
            text:"通常ボタン"
        )
        hStack(
            button(
                text:"戻る"
            )
            tonalButton(
                text:"注文"
                width:max
            )
        )
        card(
            text:"使用例"
            color:black
            text(
                text:"button\(\n\ \ text\:\"戻る\"\n\)"
            )
        )
    )

    card(
        text:"複数画面"
        button(
            text:"Screen1に移動する"
            oneClick:setScreen(screen1)
        )

    )

    card(
        text:"小計算機"
        text(
            --calc:""
            text:--calc
            id:calcDebug
            color:gray
        )
        text(
            --calc-a:0
            text:--calc-a
        )
        hStack(
            tonalButton(
                text:"1"
                oneClick:--calc=--calc+"1"
            )
            tonalButton(
                text:"2"
                oneClick:--calc=--calc+"2"
            )
            tonalButton(
                text:"3"
                oneClick:--calc=--calc+"3"
            )
            button(
                text:"+"
                oneClick:--calc=--calc+"+"
            )
        )
        hStack(
            tonalButton(
                text:"4"
                oneClick:--calc=--calc+"4"
            )
            tonalButton(
                text:"5"
                oneClick:--calc=--calc+"5"
            )
            tonalButton(
                text:"6"
                oneClick:--calc=--calc+"6"
            )
            button(
                text:"-"
                oneClick:--calc=--calc+"-"
            )
        )
        hStack(
            tonalButton(
                text:"7"
                oneClick:--calc=--calc+"7"
            )
            tonalButton(
                text:"8"
                oneClick:--calc=--calc+"8"
            )
            tonalButton(
                text:"9"
                oneClick:--calc=--calc+"9"
            )
            button(
                text:"×"
                oneClick:--calc=--calc+"*"
            )
        )
        hStack(
            tonalButton(
                text:"0"
                oneClick:--calc=--calc+"0"
            )
            button(
                text:"="
                oneClick:--calc-a=calc(--calc)
                color:yellow
            )
            tonalButton(
                text:"C"
                oneClick:--calc="",--calc-a=0
                color:yellow
            )
            button(
                text:"÷"
                oneClick:--calc=--calc+"/"
            )
        )
        button(
            --calcButtontext:"途中式を非表示"
            --calcDebugHide:false
            text:--calcButtontext
            oneClick:script(calcText)
        )
    )

    card(
        text:"要素追加"
        hStack(
            button(
                text:"追加"
                oneClick:--cb=add,script(ctrl)
            )
            button(
                text:"1つ削除"
                oneClick:--cb=del,script(ctrl)
            )
            button(
                text:"クリア"
                oneClick:--cb=clr,script(ctrl)
            )
        )
        vStack(
            id:cList
        )

    )
)
@ctrl(
    if:--cb=add(
        add(cList:'button(text:"ボタンを追加")')
    )
    if:--cb=del(
        del(cList:button)
    )
    if:--cb=clr(
        clr(cList)
    )
)

@calcText(
    if:--calcDebugHide=false(
        --calcButtontext="途中式を表示",
        hide(calcDebug),
        --calcDebugHide=true
    )
    elseIf:--calcDebugHide=true(
        --calcButtontext="途中式を非表示",
        show(calcDebug),
        --calcDebugHide=false
    )
)

screen(
    id:screen1

    Header(
    text:"Screen"
    )

    text(
        text:"Screen1の内容です。"
    )

    button(
        text:"mainに戻る"
        oneClick:setScreen(main)
    )
)
        """.trimIndent()

        setContent {
            WarpTheme {
                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    Box(modifier = Modifier.padding(innerPadding)) {
                        WarpApp(code = sampleWarpCode)
                    }
                }
            }
        }
    }
}

// ==========================================
// 1. Warp State (状態管理)
// ==========================================
class WarpState {
    var currentScreen by mutableStateOf("main")
    val vars = mutableStateMapOf<String, String>()
    val visibility = mutableStateMapOf<String, Boolean>()
    val dynamicNodes = mutableStateMapOf<String, List<WarpNode.Component>>()
}

// ==========================================
// 2. Warp AST Nodes (構文木)
// ==========================================
sealed class WarpNode {
    data class Component(
        val name: String,
        val props: Map<String, List<String>>,
        val events: Map<String, String>,
        val children: List<WarpNode>
    ) : WarpNode()

    data class Script(
        val name: String,
        val blocks: List<ScriptBlock>
    ) : WarpNode()
}

data class ScriptBlock(
    val type: String,
    val condition: String,
    val actions: String
)

// ==========================================
// 3. Warp Engine (パーサー＆評価器)
// ==========================================
class WarpEngine(val code: String, val state: WarpState) {
    val ast: List<WarpNode>

    init {
        ast = parseCode(code)
        initState(ast)
    }

    private fun initState(nodes: List<WarpNode>) {
        fun walk(node: WarpNode) {
            if (node is WarpNode.Component) {
                node.props.forEach { (k, v) ->
                    if (k.startsWith("--")) {
                        if (state.vars[k] == null) {
                            state.vars[k] = evalExpr(v)
                        }
                    }
                }
                node.children.forEach { walk(it) }
            }
        }
        nodes.forEach { walk(it) }
    }

    private fun tokenize(code: String): List<String> {
        val tokens = mutableListOf<String>()
        var pos = 0
        while (pos < code.length) {
            val c = code[pos]
            if (c.isWhitespace()) {
                pos++; continue
            }
            if (c == '"' || c == '\'') {
                val quote = c
                var str = quote.toString()
                pos++
                while (pos < code.length) {
                    if (code[pos] == '\\') {
                        str += code[pos]
                        pos++
                        if (pos < code.length) {
                            str += code[pos]
                            pos++
                        }
                        continue
                    }
                    if (code[pos] == quote) {
                        str += code[pos]
                        pos++
                        break
                    }
                    str += code[pos]
                    pos++
                }
                tokens.add(str)
                continue
            }
            if (listOf('(', ')', ',', ':', '=', '+').contains(c)) {
                tokens.add(c.toString())
                pos++; continue
            }
            var word = ""
            while (pos < code.length && !code[pos].isWhitespace() && !listOf('(', ')', ',', ':', '=', '+', '"', '\'').contains(code[pos])) {
                word += code[pos]
                pos++
            }
            if (word.isNotEmpty()) tokens.add(word)
        }
        return tokens
    }

    private fun parseCode(code: String): List<WarpNode> {
        val tokens = tokenize(code)
        var pos = 0

        fun isReservedFunc(name: String): Boolean {
            return listOf("reset", "calc", "script", "add", "del", "clr", "show", "hide").contains(name) || name.startsWith("setScreen")
        }

        fun parseNode(): WarpNode? {
            if (pos >= tokens.size) return null
            val token = tokens[pos]

            if (token.startsWith("@")) {
                val name = token.substring(1)
                val blocks = mutableListOf<ScriptBlock>()
                pos += 2
                while (pos < tokens.size && tokens[pos] != ")") {
                    val blockTypeToken = tokens[pos]
                    if (blockTypeToken == "if" || blockTypeToken == "elseIf") {
                        val blockType = blockTypeToken
                        pos++
                        if (tokens[pos] == ":") pos++

                        val condTokens = mutableListOf<String>()
                        while (pos < tokens.size && tokens[pos] != "(" && tokens[pos] != ")") {
                            condTokens.add(tokens[pos])
                            pos++
                        }
                        val condition = condTokens.joinToString("")
                        if (tokens[pos] == "(") pos++

                        val actionTokens = mutableListOf<String>()
                        var parenCount = 0
                        while (pos < tokens.size) {
                            if (parenCount == 0 && tokens[pos] == ")") break
                            if (tokens[pos] == "(") parenCount++
                            if (tokens[pos] == ")") parenCount--
                            actionTokens.add(tokens[pos])
                            pos++
                        }
                        blocks.add(ScriptBlock(blockType, condition, actionTokens.joinToString("")))
                        if (pos < tokens.size && tokens[pos] == ")") pos++
                    } else {
                        pos++
                    }
                }
                pos++
                return WarpNode.Script(name, blocks)
            }

            if (pos + 1 < tokens.size && tokens[pos + 1] == "(") {
                val name = token
                val props = mutableMapOf<String, List<String>>()
                val events = mutableMapOf<String, String>()
                val children = mutableListOf<WarpNode>()
                pos += 2

                while (pos < tokens.size && tokens[pos] != ")") {
                    val t = tokens[pos]

                    if (pos + 1 < tokens.size && tokens[pos + 1] == "(" && !isReservedFunc(t)) {
                        val child = parseNode()
                        if (child != null) children.add(child)
                        continue
                    }
                    if (pos + 1 < tokens.size && tokens[pos + 1] == ":") {
                        val key = t
                        pos += 2
                        val expr = mutableListOf<String>()
                        var parenCount = 0

                        while (pos < tokens.size) {
                            val currentToken = tokens[pos]
                            if (parenCount == 0) {
                                if (currentToken == ")") break
                                if (pos + 1 < tokens.size && tokens[pos + 1] == "(" && !isReservedFunc(currentToken)) break
                                if (pos + 1 < tokens.size && tokens[pos + 1] == ":") break
                            }
                            if (currentToken == "(") parenCount++
                            else if (currentToken == ")") parenCount--
                            expr.add(currentToken)
                            pos++
                        }

                        if (key == "oneClick" || key == "longPress") {
                            events[key] = expr.joinToString("")
                        } else {
                            props[key] = expr
                        }
                        continue
                    }
                    pos++
                }
                pos++
                return WarpNode.Component(name, props, events, children)
            }
            pos++
            return null
        }

        val parsedAst = mutableListOf<WarpNode>()
        while (pos < tokens.size) {
            val node = parseNode()
            if (node != null) parsedAst.add(node)
        }
        return parsedAst
    }

    fun evalExpr(exprArr: List<String>?): String {
        if (exprArr.isNullOrEmpty()) return ""
        var result = ""
        for (t in exprArr) {
            if (t == "+") continue
            if (t.startsWith("--")) {
                result += state.vars[t] ?: ""
            } else if ((t.startsWith("\"") && t.endsWith("\"")) || (t.startsWith("'") && t.endsWith("'"))) {
                var inner = t.substring(1, t.length - 1)
                inner = inner.replace("\\n", "\n")
                    .replace("\\\"", "\"")
                    .replace("\\'", "'")
                    .replace("\\(", "(")
                    .replace("\\)", ")")
                    .replace("\\:", ":")
                    .replace("\\ ", " ")
                result += inner
            } else {
                result += t
            }
        }
        return result
    }

    private fun evaluateRHS(exprStr: String): String {
        if (exprStr.startsWith("calc(") && exprStr.endsWith(")")) {
            var inner = exprStr.substring(5, exprStr.length - 1)
            val regex = Regex("--[a-zA-Z0-9_-]+")
            inner = regex.replace(inner) { matchResult ->
                state.vars[matchResult.value] ?: "0"
            }
            return try {
                evalBasicMath(inner).toString()
            } catch (e: Exception) {
                "Error"
            }
        }

        val parts = mutableListOf<String>()
        var current = ""
        var inQuote = false
        for (c in exprStr) {
            if (c == '"' || c == '\'') {
                inQuote = !inQuote
                current += c
            } else if (c == '+' && !inQuote) {
                parts.add(current.trim())
                current = ""
            } else {
                current += c
            }
        }
        if (current.isNotEmpty()) parts.add(current.trim())

        var result = ""
        for (p in parts) {
            if ((p.startsWith("\"") && p.endsWith("\"")) || (p.startsWith("'") && p.endsWith("'"))) {
                var inner = p.substring(1, p.length - 1)
                inner = inner.replace("\\n", "\n")
                    .replace("\\\"", "\"")
                    .replace("\\'", "'")
                    .replace("\\(", "(")
                    .replace("\\)", ")")
                    .replace("\\:", ":")
                    .replace("\\ ", " ")
                result += inner
            } else if (p.startsWith("--")) {
                result += state.vars[p] ?: ""
            } else if (p.isNotEmpty()) {
                result += p
            }
        }
        return result
    }

    private fun evalBasicMath(expr: String): Int {
        val sanitized = expr.replace("\\s".toRegex(), "")
        val tokens = sanitized.split("(?<=[-+*/])|(?=[-+*/])".toRegex()).filter { it.isNotEmpty() }
        var result = tokens.firstOrNull()?.toIntOrNull() ?: 0
        var i = 1
        while (i < tokens.size) {
            val op = tokens[i]
            val nextVal = tokens.getOrNull(i + 1)?.toIntOrNull() ?: 0
            when (op) {
                "+" -> result += nextVal
                "-" -> result -= nextVal
                "*" -> result *= nextVal
                "/" -> if (nextVal != 0) result /= nextVal
            }
            i += 2
        }
        return result
    }

    private fun evaluateCondition(condStr: String): Boolean {
        val parts = condStr.split("=")
        if (parts.size == 2) {
            val left = evalExpr(listOf(parts[0].trim()))
            val right = evalExpr(listOf(parts[1].trim()))
            return left == right
        }
        return false
    }

    fun executeAction(actionStr: String) {
        val actions = mutableListOf<String>()
        var currentAct = ""
        var inQuote = false
        var parenLevel = 0

        for (char in actionStr) {
            if (char == '"' || char == '\'') inQuote = !inQuote
            if (!inQuote && char == '(') parenLevel++
            if (!inQuote && char == ')') parenLevel--

            if (char == ',' && !inQuote && parenLevel == 0) {
                actions.add(currentAct.trim())
                currentAct = ""
            } else {
                currentAct += char
            }
        }
        if (currentAct.isNotEmpty()) actions.add(currentAct.trim())

        for (actOrig in actions) {
            var act = actOrig
            val assignIdx = act.indexOf('=')
            val colonIdx = act.indexOf(':')

            if (assignIdx == -1 && colonIdx > -1 && act.startsWith("--")) {
                act = act.substring(0, colonIdx) + "=" + act.substring(colonIdx + 1)
            }

            try {
                when {
                    act.startsWith("add(") -> {
                        val inner = act.substring(4, act.length - 1)
                        val colIdxLocal = inner.indexOf(':')
                        if (colIdxLocal > -1) {
                            val targetId = inner.substring(0, colIdxLocal).trim()
                            var compStr = inner.substring(colIdxLocal + 1).trim()
                            if ((compStr.startsWith("'") && compStr.endsWith("'")) || (compStr.startsWith("\"") && compStr.endsWith("\""))) {
                                compStr = compStr.substring(1, compStr.length - 1)
                            }
                            val dynEngine = WarpEngine(compStr, state)
                            val list = state.dynamicNodes[targetId]?.toMutableList() ?: mutableListOf()
                            list.addAll(dynEngine.ast.filterIsInstance<WarpNode.Component>())
                            state.dynamicNodes[targetId] = list
                        }
                    }
                    act.startsWith("del(") -> {
                        val inner = act.substring(4, act.length - 1)
                        val colIdxLocal = inner.indexOf(':')
                        val targetId = if (colIdxLocal > -1) inner.substring(0, colIdxLocal).trim() else inner.trim()
                        val compName = if (colIdxLocal > -1) inner.substring(colIdxLocal + 1).trim() else null

                        val list = state.dynamicNodes[targetId]?.toMutableList() ?: mutableListOf()
                        if (list.isNotEmpty()) {
                            if (compName != null) {
                                val idx = list.indexOfLast { it.name == compName }
                                if (idx != -1) list.removeAt(idx)
                            } else {
                                list.removeLast()
                            }
                            state.dynamicNodes[targetId] = list
                        }
                    }
                    act.startsWith("clr(") -> {
                        val targetId = act.substring(4, act.length - 1).trim()
                        state.dynamicNodes[targetId] = emptyList()
                    }
                    act.startsWith("show(") -> {
                        val targetId = act.substring(5, act.length - 1).trim()
                        state.visibility[targetId] = true
                    }
                    act.startsWith("hide(") -> {
                        val targetId = act.substring(5, act.length - 1).trim()
                        state.visibility[targetId] = false
                    }
                    act.startsWith("script(") -> {
                        val scriptName = act.substring(7, act.length - 1).trim().removeSurrounding("'", "'").removeSurrounding("\"", "\"")
                        executeScript(scriptName)
                    }
                    act.startsWith("setScreen(") -> {
                        val targetScreen = act.substring(10, act.length - 1).trim().removeSurrounding("'", "'").removeSurrounding("\"", "\"")
                        state.currentScreen = targetScreen
                    }
                    act.startsWith("reset(") -> {
                        val current = state.currentScreen
                        state.vars.clear()
                        state.visibility.clear()
                        state.dynamicNodes.clear()
                        state.currentScreen = current
                        initState(ast)
                    }
                    act.contains("=") -> {
                        val parts = act.split("=")
                        val key = parts[0].trim()
                        val valStr = parts.drop(1).joinToString("=").trim()
                        state.vars[key] = evaluateRHS(valStr)
                    }
                }
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
    }

    private fun executeScript(scriptName: String) {
        val scriptNode = ast.find { it is WarpNode.Script && it.name == scriptName } as? WarpNode.Script ?: return
        var matchedIf = false

        for (block in scriptNode.blocks) {
            if (block.type == "if") {
                matchedIf = evaluateCondition(block.condition)
                if (matchedIf) {
                    executeAction(block.actions)
                }
            } else if (block.type == "elseIf") {
                if (!matchedIf) {
                    if (evaluateCondition(block.condition)) {
                        matchedIf = true
                        executeAction(block.actions)
                    }
                }
            }
        }
    }
}

// ==========================================
// 4. Compose UI Renderer (UI描画)
// ==========================================
@Composable
fun WarpApp(code: String) {
    val state = remember { WarpState() }
    val engine = remember(code) { WarpEngine(code, state) }

    // LazyColumnを使用して全体をスクロール可能にする
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(bottom = 80.dp) // 下部に少し余白
    ) {
        engine.ast.forEach { node ->
            if (node is WarpNode.Component && node.name == "screen") {
                val screenId = engine.evalExpr(node.props["id"])
                if (screenId == state.currentScreen) {
                    item {
                        Column(modifier = Modifier.fillMaxWidth()) {
                            node.children.forEach { child ->
                                if (child is WarpNode.Component) {
                                    WarpComponentRenderer(child, engine, state)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WarpComponentRenderer(node: WarpNode.Component, engine: WarpEngine, state: WarpState) {
    val idVal = engine.evalExpr(node.props["id"])
    if (idVal.isNotEmpty() && state.visibility[idVal] == false) return

    val colorVal = engine.evalExpr(node.props["color"])
    val componentColor = when (colorVal) {
        "yellow" -> Color(0xFFFBC02D)
        "red" -> Color(0xFFB3261E)
        "blue" -> Color(0xFF0A56D0)
        "gray" -> Color(0xFF808080)
        "black" -> Color(0xFF000000)
        else -> Color.Unspecified
    }

    when (node.name) {
        "Header" -> {
            // ネイティブライクなTopAppBarに変更
            TopAppBar(
                title = {
                    Text(
                        text = engine.evalExpr(node.props["text"]),
                        fontWeight = FontWeight.Bold
                    )
                },
                actions = {
                    node.children.forEach { child ->
                        if (child is WarpNode.Component) WarpComponentRenderer(child, engine, state)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                    titleContentColor = MaterialTheme.colorScheme.onSurface
                )
            )
        }
        "text" -> {
            Text(
                text = engine.evalExpr(node.props["text"]),
                color = if (componentColor != Color.Unspecified) componentColor else Color.Unspecified,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
            )
        }
        "card" -> {
            // OutlinedCard と薄い背景色でモダンかつネイティブライクなデザインに調整
            OutlinedCard(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)
                ),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f)),
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    val title = engine.evalExpr(node.props["text"])
                    if (title.isNotEmpty()) {
                        Text(
                            text = title,
                            fontSize = 18.sp,
                            fontWeight = FontWeight.Bold,
                            color = if (componentColor != Color.Unspecified) componentColor else Color.Unspecified,
                            modifier = Modifier.padding(bottom = 8.dp)
                        )
                    }
                    node.children.forEach { child ->
                        if (child is WarpNode.Component) WarpComponentRenderer(child, engine, state)
                    }
                }
            }
        }
        "hStack" -> {
            Row(modifier = Modifier.padding(vertical = 4.dp)) {
                node.children.forEach { child ->
                    if (child is WarpNode.Component) WarpComponentRenderer(child, engine, state)
                }
            }
        }
        "vStack" -> {
            Column(modifier = Modifier.padding(vertical = 4.dp)) {
                node.children.forEach { child ->
                    if (child is WarpNode.Component) WarpComponentRenderer(child, engine, state)
                }
            }
        }
        "button", "tonalButton" -> {
            val text = engine.evalExpr(node.props["text"])
            val isMax = engine.evalExpr(node.props["width"]) == "max"
            val modifier = if (isMax) Modifier.fillMaxWidth().padding(horizontal = 4.dp) else Modifier.padding(horizontal = 4.dp)

            val onClick: () -> Unit = {
                node.events["oneClick"]?.let { engine.executeAction(it) }
                Unit
            }
            val onLongClick: () -> Unit = {
                node.events["longPress"]?.let { engine.executeAction(it) }
                Unit
            }

            if (node.name == "tonalButton") {
                FilledTonalButton(
                    onClick = onClick,
                    modifier = modifier.pointerInput(Unit) {
                        detectTapGestures(
                            onLongPress = { _ -> onLongClick() },
                            onTap = { _ -> onClick() }
                        )
                    },
                    colors = ButtonDefaults.filledTonalButtonColors(
                        containerColor = if (componentColor != Color.Unspecified) componentColor.copy(alpha = 0.1f) else MaterialTheme.colorScheme.secondaryContainer,
                        contentColor = if (componentColor != Color.Unspecified) componentColor else MaterialTheme.colorScheme.onSecondaryContainer
                    )
                ) {
                    Text(text)
                }
            } else {
                Button(
                    onClick = onClick,
                    modifier = modifier.pointerInput(Unit) {
                        detectTapGestures(
                            onLongPress = { _ -> onLongClick() },
                            onTap = { _ -> onClick() }
                        )
                    }
                ) {
                    Text(text)
                }
            }
        }
    }

    if (idVal.isNotEmpty()) {
        state.dynamicNodes[idVal]?.forEach { dNode ->
            WarpComponentRenderer(dNode, engine, state)
        }
    }
}