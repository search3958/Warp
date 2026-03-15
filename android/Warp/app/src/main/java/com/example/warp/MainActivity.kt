package com.example.warp

import android.content.Context
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.zIndex
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.warp.ui.theme.WarpTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlin.random.Random

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // demo.warp の内容をベースにしたサンプルコード
        val sampleWarpCode = """

screen{
    id: (main),
    
    Header{
        text: ("Warp Demo"),
        
        button{
            --headerText: (""),
            text: ("⚠️" + --headerText),
            oneClick: (--headerText = "Please wait", reset{now})
        }
    },
    
    button{
        text: ("FAB"),
        frame: (width = 100vw - 40, height = 40),
        position: (bottom = 20, left = 20),
        zIndex: (10)
    },
    
    text{
        text: ("これは新しい、ネイティブとWeb技術が融合する言語です。気になる反応は" + --mainText),
        --mainText: ("...")
    },
    
    card{
        text: ("内容変更"),
        
        text{
            text: ("クリックにより表示内容が変更されます")
        },
        
        hStack{
            button{
                text: ("👍"),
                oneClick: (--mainText = "良いみたいです!")
            },
            
            button{
                text: ("🤔"),
                oneClick: (--mainText = "あまり良くないようですね。"),
                offset: (left = 10)
            },
            
            button{
                text: ("❌"),
                status: (disabled),
                oneClick: (--mainText = "最悪ですね"),
                id: (disabledButton)
            },
            
            button{
                text: ("ボタンを有効化"),
                oneClick: (disabledButton.setStatus{unset})
            }
        },
        
        button{
            text: ("Click"),
            --color: (yellow),
            color: (--color),
            oneClick: (--color = red),
            longPress: (--color = blue)
        }
    },
    
    card{
        text: ("button"),
        
        text{
            text: ("通常ボタン")
        },
        
        hStack{
            button{
                text: ("戻る")
            },
            
            tonalButton{
                text: ("注文"),
                width: (max)
            }
        },
        
        card{
            text: ("使用例"),
            color: (#091529),
            textColor: (#eeeeff),
            
            text{
                text: ("button{\n  text: (\"戻る\")\n}")
            }
        }
    },
    
    card{
        text: ("複数画面"),
        
        button{
            text: ("Screen1に移動する"),
            oneClick: (setScreen{screen1})
        }
    },
    
    card{
        text: ("入力"),
        
        switch{
            --switchMain: (falseDisabled),
            status: (--switchMain),
            output: (--switchMain)
        },
        
        text{
            text: ("スイッチの状態は" + --switchMain + "となっています")
        },
        
        button{
            text: ("スイッチを無効化"),
            oneClick: (--switchMain = --switchMain + "Disabled"
                
            )
        },
        
        button{
            text: ("スイッチを有効化"),
            oneClick: (
                --switchMain = --switchMain.replace{"Disabled", ""}
            )
        },
        
        slider{
            --sliderMain: (50),
            status: (--sliderMain),
            output: (--sliderMain),
            max: (100),
            step: (0.5)
        },
        
        text{
            text: ("スライダー値は" + --sliderMain + "です。")
        },
        
        input{
            --inputMain: ("私の名前はです。"),
            output: (--inputMain),
            placeholder: ("自己紹介文を入力"),
            id: (nameInput)
        },
        
        button{
            text: ("保存"),
            oneClick: (~~name = --inputMain, --saveStatus = "保存完了", wait: (1), --saveStatus = null)
        },
        
        button{
            text: ("復元"),
            oneClick: (nameInput.changeContent{~~name}, --saveStatus = "復元完了", wait: (1), --saveStatus = null)
        },
        
        text{
            --saveStatus: (null),
            text: (--saveStatus)
        }
    },
    
    card{
        text: ("小計算機"),
        
        text{
            --calc: (""),
            text: (--calc),
            id: (calcDebug),
            textColor: (gray)
        },
        
        text{
            --calc-a: (0),
            text: (--calc-a)
        },
        
        hStack{
            tonalButton{
                text: ("1"),
                oneClick: (--calc = --calc + "1")
            },
            
            tonalButton{
                text: ("2"),
                oneClick: (--calc = --calc + "2")
            },
            
            tonalButton{
                text: ("3"),
                oneClick: (--calc = --calc + "3")
            },
            
            button{
                text: ("+"),
                oneClick: (--calc = --calc + "+")
            }
        },
        
        hStack{
            tonalButton{
                text: ("4"),
                oneClick: (--calc = --calc + "4")
            },
            
            tonalButton{
                text: ("5"),
                oneClick: (--calc = --calc + "5")
            },
            
            tonalButton{
                text: ("6"),
                oneClick: (--calc = --calc + "6")
            },
            
            button{
                text: ("-"),
                oneClick: (--calc = --calc + "-")
            }
        },
        
        hStack{
            tonalButton{
                text: ("7"),
                oneClick: (--calc = --calc + "7")
            },
            
            tonalButton{
                text: ("8"),
                oneClick: (--calc = --calc + "8")
            },
            
            tonalButton{
                text: ("9"),
                oneClick: (--calc = --calc + "9")
            },
            
            button{
                text: ("×"),
                oneClick: (--calc = --calc + "*")
            }
        },
        
        hStack{
            tonalButton{
                text: ("0"),
                oneClick: (--calc = --calc + "0")
            },
            
            button{
                text: ("="),
                oneClick: (--calc-a = calc{--calc}),
                color: (yellow)
            },
            
            tonalButton{
                text: ("C"),
                oneClick: (--calc = "0", --calc-a = 0),
                color: (yellow)
            },
            
            button{
                text: ("÷"),
                oneClick: (--calc = --calc + "/")
            }
        },
        
        button{
            --calcButtonText: ("途中式を非表示"),
            --calcDebugHide: (false),
            text: (--calcButtonText),
            oneClick: (script{calcText})
        }
    },
    
    card{
        text: ("要素追加"),
        
        hStack{
            button{
                text: ("追加"),
                oneClick: (--cb = "add", script{ctrl})
            },
            
            button{
                text: ("1つ削除"),
                oneClick: (--cb = "del", script{ctrl})
            },
            
            button{
                text: ("クリア"),
                oneClick: (--cb = "clr", script{ctrl})
            }
        },
        
        vStack{
            id: (cList)
        }
    }
}

@ctrl{
    if: (--cb = "add") {
        add{cList: ("button{text: ('ボタンを追加')}")}
    },
    
    if: (--cb = "del") {
        del{cList: (button)}
    },
    
    if: (--cb = "clr") {
        clr{cList}
    }
}

@calcText{
    if: (--calcDebugHide = false) {
        --calcButtonText = "途中式を表示",
        hide{calcDebug},
        --calcDebugHide = true
    },
    
    elseIf: (--calcDebugHide = true) {
        --calcButtonText = "途中式を非表示",
        show{calcDebug},
        --calcDebugHide = false
    }
}

screen{
    id: (screen1),
    
    Header{
        text: ("Screen")
    },
    
    text{
        text: ("Screen1の内容です。")
    },
    
    button{
        text: ("mainに戻る"),
        oneClick: (setScreen{main})
    }
}
        """.trimIndent()

        setContent {
            WarpTheme {
                Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                    WarpApp(code = sampleWarpCode)
                }
            }
        }
    }
}

// ==========================================
// 1. Warp State (状態管理)
// ==========================================
class WarpState(context: Context) {
    var currentScreen by mutableStateOf("main")
    val vars = mutableStateMapOf<String, String>()
    val visibility = mutableStateMapOf<String, Boolean>()
    val status = mutableStateMapOf<String, String>()
    val dynamicNodes = mutableStateMapOf<String, List<WarpNode.Component>>()

    private val prefs = context.getSharedPreferences("warp_prefs", Context.MODE_PRIVATE)

    fun getVar(name: String): String {
        val cached = vars[name]
        if (cached != null) return cached

        return if (name.startsWith("~~")) {
            val saved = prefs.getString(name, "") ?: ""
            vars[name] = saved
            saved
        } else {
            ""
        }
    }

    fun setVar(name: String, value: String) {
        vars[name] = value
        if (name.startsWith("~~")) {
            prefs.edit().putString(name, value).apply()
        }
    }
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
                    if (k.startsWith("--") || k.startsWith("~~")) {
                        if (state.getVar(k).isEmpty()) {
                            state.setVar(k, evalExpr(v))
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
            if (c == '/' && pos + 1 < code.length && code[pos + 1] == '/') {
                while (pos < code.length && code[pos] != '\n') pos++
                continue
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

            // Fix: Check for -- and ~~ before splitting on separators
            if ((c == '-' && pos + 1 < code.length && code[pos + 1] == '-') ||
                (c == '~' && pos + 1 < code.length && code[pos + 1] == '~')) {
                var word = c.toString() + code[pos+1].toString()
                pos += 2
                while (pos < code.length && !code[pos].isWhitespace() && !listOf('(', ')', '{', '}', ',', ':', '=', '+', '-', '*', '/', '"', '\'', ';').contains(code[pos])) {
                    word += code[pos]
                    pos++
                }
                tokens.add(word)
                continue
            }

            if (listOf('(', ')', '{', '}', ',', ':', '=', '+', '-', '*', '/', ';').contains(c)) {
                tokens.add(c.toString())
                pos++; continue
            }
            var word = ""
            while (pos < code.length && !code[pos].isWhitespace() && !listOf('(', ')', '{', '}', ',', ':', '=', '+', '-', '*', '/', '"', '\'', ';').contains(code[pos])) {
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
            return listOf("reset", "calc", "script", "add", "del", "clr", "show", "hide", "random", "wait").contains(name) || name.startsWith("setScreen")
        }

        fun parseNode(): WarpNode? {
            if (pos >= tokens.size) return null
            val token = tokens[pos]

            // Script parser (@name)
            if (token.startsWith("@")) {
                val name = token.substring(1)
                val blocks = mutableListOf<ScriptBlock>()
                pos++
                // Handle both @name(...) and @name{...}
                val endToken = if (pos < tokens.size && tokens[pos] == "{") "}" else ")"
                pos++

                while (pos < tokens.size && tokens[pos] != endToken) {
                    val blockTypeToken = tokens[pos]
                    if (blockTypeToken == "if" || blockTypeToken == "elseIf") {
                        val blockType = blockTypeToken
                        pos++
                        if (pos < tokens.size && tokens[pos] == ":") pos++

                        val condTokens = mutableListOf<String>()
                        // Support both if:(...) and if(...)
                        val condOpen = if (pos < tokens.size && tokens[pos] == "(") "(" else ""
                        if (condOpen.isNotEmpty()) pos++
                        
                        var condParenCount = if (condOpen.isNotEmpty()) 1 else 0
                        while (pos < tokens.size) {
                            val ct = tokens[pos]
                            if (condOpen.isNotEmpty()) {
                                if (ct == "(") condParenCount++
                                if (ct == ")") condParenCount--
                                if (condParenCount == 0) { pos++; break }
                            } else {
                                if (ct == "{" || ct == "(" || ct == endToken) break
                            }
                            condTokens.add(ct)
                            pos++
                        }
                        val condition = condTokens.joinToString("")

                        val blockEndToken = if (pos < tokens.size && tokens[pos] == "{") "}" else ")"
                        pos++

                        val actionTokens = mutableListOf<String>()
                        var parenCount = 1
                        while (pos < tokens.size) {
                            if (tokens[pos] == "{" || tokens[pos] == "(") parenCount++
                            if (tokens[pos] == "}" || tokens[pos] == ")") parenCount--
                            if (parenCount == 0) break
                            actionTokens.add(tokens[pos])
                            pos++
                        }
                        blocks.add(ScriptBlock(blockType, condition, actionTokens.joinToString("")))
                        if (pos < tokens.size) pos++ // skip endToken
                    } else {
                        pos++
                    }
                    if (pos < tokens.size && tokens[pos] == ",") pos++
                }
                if (pos < tokens.size) pos++
                return WarpNode.Script(name, blocks)
            }

            // Inline if: cond (...)
            if (token == "if" && pos + 1 < tokens.size && tokens[pos + 1] == ":") {
                val name = "if"
                pos += 2
                val condTokens = mutableListOf<String>()
                while (pos < tokens.size && tokens[pos] != "(" && tokens[pos] != "{") {
                    condTokens.add(tokens[pos])
                    pos++
                }
                val props = mutableMapOf<String, List<String>>()
                props["condition"] = condTokens

                val children = mutableListOf<WarpNode>()
                val endToken = if (pos < tokens.size && tokens[pos] == "{") "}" else ")"
                pos++
                while (pos < tokens.size && tokens[pos] != endToken) {
                    val child = parseNode()
                    if (child != null) children.add(child)
                    else if (tokens[pos] != endToken && tokens[pos] != ",") pos++
                    if (pos < tokens.size && tokens[pos] == ",") pos++
                }
                if (pos < tokens.size) pos++
                return WarpNode.Component(name, props, emptyMap(), children)
            }

            // Standard Component: Name ( props ) or Name { props }
            if (pos + 1 < tokens.size && (tokens[pos + 1] == "(" || tokens[pos + 1] == "{")) {
                val name = token
                val openToken = tokens[pos + 1]
                val closeToken = if (openToken == "(") ")" else "}"
                val props = mutableMapOf<String, List<String>>()
                val events = mutableMapOf<String, String>()
                val children = mutableListOf<WarpNode>()
                pos += 2

                while (pos < tokens.size && tokens[pos] != closeToken) {
                    val t = tokens[pos]

                    // Child component
                    if (pos + 1 < tokens.size && (tokens[pos + 1] == "(" || tokens[pos + 1] == "{") && !isReservedFunc(t)) {
                        val child = parseNode()
                        if (child != null) children.add(child)
                        if (pos < tokens.size && tokens[pos] == ",") pos++
                        continue
                    }
                    // Property or Event
                    else if (pos + 1 < tokens.size && tokens[pos + 1] == ":") {
                        val key = t
                        pos += 2
                        val expr = mutableListOf<String>()

                        // Handle key: (value) or key: {value} style
                        val valOpen = if (pos < tokens.size && (tokens[pos] == "(" || tokens[pos] == "{")) tokens[pos] else ""
                        val valClose = if (valOpen == "(") ")" else if (valOpen == "{") "}" else ""
                        if (valOpen.isNotEmpty()) pos++

                        var parenCount = if (valOpen.isNotEmpty()) 1 else 0

                        while (pos < tokens.size) {
                            val currentToken = tokens[pos]
                            if (valOpen.isNotEmpty()) {
                                if (currentToken == valOpen) parenCount++
                                else if (currentToken == valClose) parenCount--
                                
                                if (parenCount == 0) {
                                    pos++ // skip the closing bracket
                                    break
                                }
                            } else {
                                if (currentToken == closeToken || currentToken == ",") break
                                if (pos + 1 < tokens.size && (tokens[pos + 1] == "(" || tokens[pos + 1] == "{") && !isReservedFunc(currentToken)) break
                                if (pos + 1 < tokens.size && tokens[pos + 1] == ":") break
                            }
                            expr.add(currentToken)
                            pos++
                        }

                        if (key == "oneClick" || key == "longPress" || key == "onValueChange" || key == "output") {
                            events[key] = expr.joinToString("")
                        } else {
                            props[key] = expr
                        }
                        if (pos < tokens.size && tokens[pos] == ",") pos++
                        continue
                    }
                    pos++ // Skip unknown tokens
                }
                if (pos < tokens.size) pos++
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
            val trimmed = t.trim()
            if (trimmed == "+" || trimmed.isEmpty()) continue
            
            if (trimmed.startsWith("--") || trimmed.startsWith("~~")) {
                result += state.getVar(trimmed)
            } else if ((trimmed.startsWith("\"") && trimmed.endsWith("\"")) || (trimmed.startsWith("'") && trimmed.endsWith("'"))) {
                var inner = trimmed.substring(1, trimmed.length - 1)
                inner = inner.replace("\\n", "\n")
                    .replace("\\\"", "\"")
                    .replace("\\'", "'")
                    .replace("\\(", "(")
                    .replace("\\)", ")")
                    .replace("\\:", ":")
                    .replace("\\ ", " ")
                result += inner
            } else if (trimmed == "null") {
                // skip
            } else {
                // If it's a number or something else, but doesn't look like a separator
                if (!listOf("(", ")", "{", "}", ",", ":", "=", "+", "-", "*", "/", ";").contains(trimmed)) {
                    result += trimmed
                }
            }
        }
        return result
    }

    fun evaluateRHS(exprStr: String): String {
        var currentExpr = exprStr.trim()
        if (currentExpr.startsWith("(") && currentExpr.endsWith(")")) currentExpr = currentExpr.substring(1, currentExpr.length - 1).trim()
        if (currentExpr == "null" || currentExpr.isEmpty()) return ""
        if (currentExpr == "true") return "true"
        if (currentExpr == "false") return "false"

        // Handle functions like calc{}, replace{}, random{}, contains{}
        if (currentExpr.startsWith("calc{") || currentExpr.startsWith("calc(")) {
            val open = if (currentExpr.startsWith("calc{")) '{' else '('
            val close = if (open == '{') '}' else ')'
            var inner = currentExpr.substring(5, currentExpr.lastIndexOf(close))
            val regex = Regex("(--|~~)[a-zA-Z0-9_-]+")
            inner = regex.replace(inner) { matchResult ->
                state.getVar(matchResult.value).ifEmpty { "0" }
            }
            return try { evalBasicMath(inner).toString().replace(".0", "") } catch (e: Exception) { "0" }
        }

        if (currentExpr.contains(".replace{") || currentExpr.contains(".replace(")) {
            val idx = if (currentExpr.contains(".replace{")) currentExpr.indexOf(".replace{") else currentExpr.indexOf(".replace(")
            val open = currentExpr[idx + 8]
            val close = if (open == '{') '}' else ')'
            val base = currentExpr.substring(0, idx).trim()
            val argsStr = currentExpr.substring(idx + 9, currentExpr.lastIndexOf(close))
            val args = splitByComma(argsStr)
            if (args.size >= 2) {
                val old = evaluateRHS(args[0])
                val new = evaluateRHS(args[1])
                return evaluateRHS(base).replace(old, new)
            }
        }

        if (currentExpr.startsWith("random{") || currentExpr.startsWith("random(")) {
            val open = if (currentExpr.startsWith("random{")) '{' else '('
            val close = if (open == '{') '}' else ')'
            val args = splitByComma(currentExpr.substring(7, currentExpr.lastIndexOf(close)))
            if (args.size == 2) {
                val min = evaluateRHS(args[0]).toIntOrNull() ?: 0
                val max = evaluateRHS(args[1]).toIntOrNull() ?: 100
                return Random.nextInt(min, max + 1).toString()
            }
        }

        // Simple concatenation logic
        val parts = mutableListOf<String>()
        var current = ""
        var inQuote = false
        var pLevel = 0
        for (c in currentExpr) {
            if (c == '"' || c == '\'') inQuote = !inQuote
            if (!inQuote) {
                if (c == '(' || c == '{') pLevel++
                if (c == ')' || c == '}') pLevel--
            }
            if (c == '+' && !inQuote && pLevel == 0) {
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
                inner = inner.replace("\\n", "\n").replace("\\\"", "\"").replace("\\'", "'")
                result += inner
            } else if (p.startsWith("--") || p.startsWith("~~")) {
                result += state.getVar(p)
            } else {
                result += p
            }
        }
        return result
    }

    private fun splitByComma(s: String): List<String> {
        val res = mutableListOf<String>()
        var curr = ""
        var inQuote = false
        var pLevel = 0
        for (c in s) {
            if (c == '"' || c == '\'') inQuote = !inQuote
            if (!inQuote) {
                if (c == '(' || c == '{') pLevel++
                if (c == ')' || c == '}') pLevel--
            }
            if (c == ',' && !inQuote && pLevel == 0) {
                res.add(curr.trim())
                curr = ""
            } else {
                curr += c
            }
        }
        if (curr.isNotEmpty()) res.add(curr.trim())
        return res
    }

    private fun evalBasicMath(expr: String): Double {
        val sanitized = expr.replace("\\s".toRegex(), "")
        // Use a simple evaluator for basic math
        val tokens = sanitized.split("(?<=[-+*/])|(?=[-+*/])".toRegex()).filter { it.isNotEmpty() }
        if (tokens.isEmpty()) return 0.0
        var result = tokens.first().toDoubleOrNull() ?: 0.0
        var i = 1
        while (i < tokens.size) {
            val op = tokens[i]
            val nextVal = tokens.getOrNull(i + 1)?.toDoubleOrNull() ?: 0.0
            when (op) {
                "+" -> result += nextVal
                "-" -> result -= nextVal
                "*" -> result *= nextVal
                "/" -> if (nextVal != 0.0) result /= nextVal
            }
            i += 2
        }
        return result
    }

    fun evaluateCondition(condStr: String): Boolean {
        val trimmed = condStr.trim()
        if (trimmed.isEmpty()) return false
        
        // Handle .contains
        if (trimmed.contains(".contains{") || trimmed.contains(".contains(")) {
            val idx = if (trimmed.contains(".contains{")) trimmed.indexOf(".contains{") else trimmed.indexOf(".contains(")
            val open = trimmed[idx + 9]
            val close = if (open == '{') '}' else ')'
            val base = evaluateRHS(trimmed.substring(0, idx).trim())
            val search = evaluateRHS(trimmed.substring(idx + 10, trimmed.lastIndexOf(close)).trim())
            val has = base.contains(search)
            return if (trimmed.contains("= false")) !has else has
        }
        
        if (trimmed.contains("=")) {
            val parts = trimmed.split("=")
            val left = evaluateRHS(parts[0].trim())
            val right = evaluateRHS(parts.drop(1).joinToString("=").trim())
            return left == right
        }
        
        val eval = evaluateRHS(trimmed)
        return eval == "true" || (eval.isNotEmpty() && eval != "false")
    }

    fun executeAction(actionStr: String, scope: kotlinx.coroutines.CoroutineScope) {
        val actions = mutableListOf<String>()
        var currentAct = ""
        var inQuote = false
        var pCurly = 0
        var pRound = 0

        for (char in actionStr) {
            if (char == '"' || char == '\'') inQuote = !inQuote
            if (!inQuote) {
                if (char == '{') pCurly++
                if (char == '}') pCurly--
                if (char == '(') pRound++
                if (char == ')') pRound--
            }

            if (char == ',' && !inQuote && pCurly == 0 && pRound == 0) {
                actions.add(currentAct.trim())
                currentAct = ""
            } else {
                currentAct += char
            }
        }
        if (currentAct.isNotEmpty()) actions.add(currentAct.trim())

        scope.launch {
            for (actOrig in actions) {
                var act = actOrig
                try {
                    when {
                        act.startsWith("if:") -> {
                            val p = act.indexOf('(')
                            var depth = 0
                            var condEnd = -1
                            for (i in p until act.length) {
                                if (act[i] == '(') depth++
                                if (act[i] == ')') {
                                    depth--
                                    if (depth == 0) {
                                        condEnd = i
                                        break
                                    }
                                }
                            }
                            val cond = act.substring(p + 1, condEnd).trim()
                            val aStart = act.indexOf('{', condEnd)
                            var aDepth = 0
                            var aEnd = -1
                            for (i in aStart until act.length) {
                                if (act[i] == '{') aDepth++
                                if (act[i] == '}') {
                                    aDepth--
                                    if (aDepth == 0) {
                                        aEnd = i
                                        break
                                    }
                                }
                            }
                            val inner = act.substring(aStart + 1, aEnd)
                            if (evaluateCondition(cond)) executeAction(inner, scope)
                        }
                        act.startsWith("wait:") -> {
                            val valStr = act.split("wait:")[1].trim()
                            val time = evaluateRHS(valStr).toFloatOrNull() ?: 0f
                            delay((time * 1000).toLong())
                        }
                        act.startsWith("add{") || act.startsWith("add(") -> {
                            val inner = act.substring(4, act.length - 1)
                            val colIdxLocal = inner.indexOf(':')
                            if (colIdxLocal > -1) {
                                val targetId = inner.substring(0, colIdxLocal).trim()
                                var compStr = inner.substring(colIdxLocal + 1).trim()
                                compStr = evaluateRHS(compStr)
                                val dynEngine = WarpEngine(compStr, state)
                                val list = state.dynamicNodes[targetId]?.toMutableList() ?: mutableListOf()
                                list.addAll(dynEngine.ast.filterIsInstance<WarpNode.Component>())
                                state.dynamicNodes[targetId] = list
                            }
                        }
                        act.startsWith("del{") || act.startsWith("del(") -> {
                            val inner = act.substring(4, act.length - 1)
                            val colIdxLocal = inner.indexOf(':')
                            val targetId = if (colIdxLocal > -1) inner.substring(0, colIdxLocal).trim() else inner.trim()
                            val list = state.dynamicNodes[targetId]?.toMutableList() ?: mutableListOf()
                            if (list.isNotEmpty()) {
                                list.removeAt(list.size - 1)
                                state.dynamicNodes[targetId] = list
                            }
                        }
                        act.startsWith("clr{") || act.startsWith("clr(") -> {
                            val targetId = act.substring(4, act.length - 1).trim()
                            state.dynamicNodes[targetId] = emptyList()
                        }
                        act.startsWith("show{") || act.startsWith("show(") -> {
                            val targetId = act.substring(5, act.length - 1).trim()
                            state.visibility[targetId] = true
                        }
                        act.startsWith("hide{") || act.startsWith("hide(") -> {
                            val targetId = act.substring(5, act.length - 1).trim()
                            state.visibility[targetId] = false
                        }
                        act.startsWith("script{") || act.startsWith("script(") -> {
                            val scriptName = act.substring(7, act.length - 1).trim().removeSurrounding("'", "'").removeSurrounding("\"", "\"")
                            executeScript(scriptName, scope)
                        }
                        act.startsWith("setScreen{") || act.startsWith("setScreen(") -> {
                            val targetScreen = act.substring(10, act.length - 1).trim().removeSurrounding("'", "'").removeSurrounding("\"", "\"")
                            state.currentScreen = targetScreen
                        }
                        act.startsWith("reset{") || act.startsWith("reset(") -> {
                            // In Android, we can just clear state and re-init
                            state.vars.clear()
                            state.visibility.clear()
                            state.status.clear()
                            state.dynamicNodes.clear()
                            state.currentScreen = "main"
                            initState(ast)
                        }
                        act.contains(".setStatus{") || act.contains(".setStatus(") -> {
                            val sep = if (act.contains(".setStatus{")) ".setStatus{" else ".setStatus("
                            val targetId = act.substringBefore(sep).trim()
                            val value = act.substringAfter(sep).substringBeforeLast(if (sep.endsWith("{")) "}" else ")").trim()
                            state.status[targetId] = evaluateRHS(value)
                        }
                        act.contains(".changeContent{") || act.contains(".changeContent(") -> {
                            val sep = if (act.contains(".changeContent{")) ".changeContent{" else ".changeContent("
                            val targetId = act.substringBefore(sep).trim()
                            val value = act.substringAfter(sep).substringBeforeLast(if (sep.endsWith("{")) "}" else ")").trim()
                            state.setVar(targetId, evaluateRHS(value))
                        }
                        act.contains("=") -> {
                            val parts = act.split("=")
                            val key = parts[0].trim()
                            val valStr = parts.drop(1).joinToString("=").trim()
                            state.setVar(key, evaluateRHS(valStr))
                        }
                    }
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }
        }
    }

    private fun executeScript(scriptName: String, scope: kotlinx.coroutines.CoroutineScope) {
        val scriptNode = ast.find { it is WarpNode.Script && it.name == scriptName } as? WarpNode.Script ?: return
        var matchedIf = false

        for (block in scriptNode.blocks) {
            if (block.type == "if") {
                matchedIf = evaluateCondition(block.condition)
                if (matchedIf) {
                    executeAction(block.actions, scope)
                }
            } else if (block.type == "elseIf") {
                if (!matchedIf) {
                    if (evaluateCondition(block.condition)) {
                        matchedIf = true
                        executeAction(block.actions, scope)
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
    val context = LocalContext.current
    val state = remember { WarpState(context) }
    val engine = remember(code) { WarpEngine(code, state) }

    val currentScreenNode = engine.ast.filterIsInstance<WarpNode.Component>()
        .find { it.name == "screen" && engine.evalExpr(it.props["id"]) == state.currentScreen }

    if (currentScreenNode != null) {
        val headerNode = currentScreenNode.children.filterIsInstance<WarpNode.Component>().find { it.name == "Header" }
        val children = currentScreenNode.children.filterIsInstance<WarpNode.Component>().filter { it.name != "Header" }
        
        // Split children: normal vs fixed (has position prop)
        val fixedChildren = children.filter { it.props.containsKey("position") }
        val normalChildren = children.filter { !it.props.containsKey("position") }

        Scaffold(
            topBar = {
                headerNode?.let { WarpComponentRenderer(it, engine, state) }
            },
            modifier = Modifier.fillMaxSize()
        ) { innerPadding ->
            Box(modifier = Modifier.padding(innerPadding).fillMaxSize()) {
                // Main content
                Column(modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
                    normalChildren.forEach { child ->
                        WarpComponentRenderer(child, engine, state)
                    }
                }
                
                // Fixed/FAB content
                fixedChildren.forEach { child ->
                    WarpComponentRenderer(child, engine, state)
                }
            }
        }
    } else {
        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Text("Screen '${state.currentScreen}' not found")
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WarpComponentRenderer(
    node: WarpNode.Component,
    engine: WarpEngine,
    state: WarpState,
    parentModifier: Modifier = Modifier
) {
    val idVal = engine.evalExpr(node.props["id"])
    if (idVal.isNotEmpty() && state.visibility[idVal] == false) return

    val currentStatus = if (idVal.isNotEmpty()) state.status[idVal] ?: engine.evalExpr(node.props["status"]) else engine.evalExpr(node.props["status"])
    val isEnabled = currentStatus != "disabled" && !currentStatus.contains("Disabled")

    val coroutineScope = rememberCoroutineScope()
    
    val config = LocalConfiguration.current
    val sw = config.screenWidthDp
    val sh = config.screenHeightDp

    fun resolveViewport(s: String): String {
        return s.replace("vw", "*$sw/100").replace("vh", "*$sh/100")
    }

    // Common Style Props
    val padding = engine.evalExpr(node.props["padding"]).toIntOrNull() ?: 0
    val gap = engine.evalExpr(node.props["gap"]).toIntOrNull() ?: 8
    val opacity = engine.evalExpr(node.props["opacity"]).toFloatOrNull() ?: 1f
    val bgColorStr = engine.evalExpr(node.props["background"]) ?: engine.evalExpr(node.props["color"])
    val cornerRadius = engine.evalExpr(node.props["cornerRadius"]).toIntOrNull() ?: 0
    val zIndexVal = engine.evalExpr(node.props["zIndex"]).toFloatOrNull() ?: 0f
    
    // Layout Props
    val widthVal = engine.evalExpr(node.props["width"])
    val heightVal = engine.evalExpr(node.props["height"])
    
    var modifier: Modifier = parentModifier
        .alpha(opacity)
        .zIndex(zIndexVal)
        .then(if (padding > 0) Modifier.padding(padding.dp) else Modifier)
        .then(if (bgColorStr.isNotEmpty()) Modifier.background(
            color = parseColor(bgColorStr),
            shape = RoundedCornerShape(cornerRadius.dp)
        ) else Modifier)

    // Handle frame, offset, position
    val frameVal = node.props["frame"]?.joinToString("") ?: ""
    if (frameVal.isNotEmpty()) {
        frameVal.split(",").forEach { s ->
            val parts = s.split("=")
            if (parts.size == 2) {
                val k = parts[0].trim()
                val vStr = resolveViewport(parts[1].trim())
                val v = engine.evaluateRHS("calc($vStr)").replace(".0", "").toIntOrNull() ?: 0
                if (k == "width") modifier = modifier.width(v.dp)
                if (k == "height") modifier = modifier.height(v.dp)
            }
        }
    }

    val offsetVal = node.props["offset"]?.joinToString("") ?: ""
    if (offsetVal.isNotEmpty()) {
        var top = 0; var left = 0; var right = 0; var bottom = 0
        offsetVal.split(",").forEach { s ->
            val parts = s.split("=")
            if (parts.size == 2) {
                val k = parts[0].trim()
                val vStr = resolveViewport(parts[1].trim())
                val v = engine.evaluateRHS("calc($vStr)").replace(".0", "").toIntOrNull() ?: 0
                when (k) {
                    "top" -> top = v
                    "left" -> left = v
                    "right" -> right = v
                    "bottom" -> bottom = v
                }
            }
        }
        modifier = modifier.padding(start = left.dp, top = top.dp, end = right.dp, bottom = bottom.dp)
    }

    val positionVal = node.props["position"]?.joinToString("") ?: ""
    if (positionVal.isNotEmpty()) {
        var top: Int? = null; var left: Int? = null; var right: Int? = null; var bottom: Int? = null
        positionVal.split(",").forEach { s ->
            val parts = s.split("=")
            if (parts.size == 2) {
                val k = parts[0].trim()
                val vStr = resolveViewport(parts[1].trim())
                val v = engine.evaluateRHS("calc($vStr)").replace(".0", "").toIntOrNull() ?: 0
                when (k) {
                    "top" -> top = v
                    "bottom" -> bottom = v
                    "left" -> left = v
                    "right" -> right = v
                }
            }
        }
        
        // Map top/bottom/left/right to alignment + offset
        if (top != null) modifier = modifier.offset(y = top!!.dp)
        if (bottom != null) {
            // If inside a Box (Fixed components are inside Box), we want to align bottom
            // But for simplicity with current architecture, let's use screenHeight - bottom as offset from top
            modifier = modifier.offset(y = (sh - bottom!! - 40).dp) // -40 as approx height for now, refined below
        }
        if (left != null) modifier = modifier.offset(x = left!!.dp)
        if (right != null) {
             modifier = modifier.offset(x = (sw - right!! - 100).dp) // -100 as approx width
        }
    }
    
    if (widthVal == "max") {
        modifier = modifier.fillMaxWidth()
    } else if (widthVal.isNotEmpty()) {
        val wStr = resolveViewport(widthVal)
        val w = if (wStr.contains("*")) engine.evaluateRHS("calc($wStr)").replace(".0", "").toIntOrNull() else wStr.toIntOrNull()
        if (w != null) modifier = modifier.width(w.dp)
    }

    if (heightVal == "max") {
        modifier = modifier.fillMaxHeight()
    } else if (heightVal.isNotEmpty()) {
        val hStr = resolveViewport(heightVal)
        val h = if (hStr.contains("*")) engine.evaluateRHS("calc($hStr)").replace(".0", "").toIntOrNull() else hStr.toIntOrNull()
        if (h != null) modifier = modifier.height(h.dp)
    }

    when (node.name) {
        "if" -> {
            val condition = engine.evalExpr(node.props["condition"])
            if (engine.evaluateCondition(condition)) {
                node.children.forEach { child ->
                    if (child is WarpNode.Component) WarpComponentRenderer(child, engine, state)
                }
            }
        }
        "Header" -> {
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
                }
            )
        }
        "scrollView" -> {
            Column(modifier = modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
                node.children.forEach { child ->
                    if (child is WarpNode.Component) {
                        val childWidth = engine.evalExpr(child.props["width"])
                        val childModifier = if (childWidth == "max") Modifier.fillMaxWidth() else Modifier
                        WarpComponentRenderer(child, engine, state, childModifier)
                    }
                }
            }
        }
        "vStack" -> {
            Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(gap.dp)) {
                node.children.forEach { child ->
                    if (child is WarpNode.Component) {
                        val childHeight = engine.evalExpr(child.props["height"])
                        val childWidth = engine.evalExpr(child.props["width"])
                        var childModifier: Modifier = Modifier
                        if (childHeight == "max") childModifier = childModifier.weight(1f)
                        if (childWidth == "max") childModifier = childModifier.fillMaxWidth()
                        WarpComponentRenderer(child, engine, state, childModifier)
                    }
                }
            }
        }
        "hStack" -> {
            Row(modifier = modifier, horizontalArrangement = Arrangement.spacedBy(gap.dp), verticalAlignment = Alignment.CenterVertically) {
                node.children.forEach { child ->
                    if (child is WarpNode.Component) {
                        val childWidth = engine.evalExpr(child.props["width"])
                        val childHeight = engine.evalExpr(child.props["height"])
                        var childModifier: Modifier = Modifier
                        if (childWidth == "max") childModifier = childModifier.weight(1f)
                        if (childHeight == "max") childModifier = childModifier.fillMaxHeight()
                        WarpComponentRenderer(child, engine, state, childModifier)
                    }
                }
            }
        }
        "text" -> {
            val colorVal = engine.evalExpr(node.props["color"]) ?: engine.evalExpr(node.props["textColor"])
            val fontSize = engine.evalExpr(node.props["fontSize"]).toIntOrNull() ?: 16
            val fontWeightStr = engine.evalExpr(node.props["fontWeight"])
            val alignStr = engine.evalExpr(node.props["align"])

            Text(
                text = engine.evalExpr(node.props["text"]),
                color = parseColor(colorVal),
                fontSize = fontSize.sp,
                fontWeight = if (fontWeightStr == "bold") FontWeight.Bold else FontWeight.Normal,
                textAlign = when (alignStr) {
                    "center" -> TextAlign.Center
                    "trailing" -> TextAlign.End
                    else -> TextAlign.Start
                },
                modifier = modifier.fillMaxWidth().padding(vertical = 4.dp)
            )
        }
        "card" -> {
            OutlinedCard(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.3f)
                ),
                border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f)),
                modifier = modifier.fillMaxWidth().padding(vertical = 8.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    val title = engine.evalExpr(node.props["text"])
                    if (title.isNotEmpty()) {
                        Text(text = title, fontSize = 18.sp, fontWeight = FontWeight.Bold, modifier = Modifier.padding(bottom = 8.dp))
                    }
                    node.children.forEach { child ->
                        if (child is WarpNode.Component) {
                            val childHeight = engine.evalExpr(child.props["height"])
                            val childModifier: Modifier = if (childHeight == "max") Modifier.weight(1f) else Modifier
                            WarpComponentRenderer(child, engine, state, childModifier)
                        }
                    }
                }
            }
        }
        "button", "tonalButton" -> {
            val text = engine.evalExpr(node.props["text"])
            val isTonal = node.name == "tonalButton" || engine.evalExpr(node.props["tonalButton"]) == "true"

            val onClick = { node.events["oneClick"]?.let { engine.executeAction(it, coroutineScope) }; Unit }
            val onLongClick = { node.events["longPress"]?.let { engine.executeAction(it, coroutineScope) }; Unit }

            if (isTonal) {
                FilledTonalButton(
                    onClick = onClick,
                    enabled = isEnabled,
                    modifier = modifier.pointerInput(Unit) { detectTapGestures(onLongPress = { if(isEnabled) onLongClick() }, onTap = { if(isEnabled) onClick() }) }
                ) { Text(text) }
            } else {
                Button(
                    onClick = onClick,
                    enabled = isEnabled,
                    modifier = modifier.pointerInput(Unit) { detectTapGestures(onLongPress = { if(isEnabled) onLongClick() }, onTap = { if(isEnabled) onClick() }) }
                ) { Text(text) }
            }
        }
        "textField", "input" -> {
            val varName = node.props["text"]?.joinToString("") ?: node.events["output"] ?: ""
            val textValue = state.vars[varName] ?: state.getVar(varName)

            OutlinedTextField(
                value = textValue,
                enabled = isEnabled,
                onValueChange = {
                    state.setVar(varName, it)
                },
                placeholder = { Text(engine.evalExpr(node.props["placeholder"])) },
                modifier = modifier.fillMaxWidth().padding(vertical = 4.dp)
            )
        }
        "toggle", "switch" -> {
            val varName = node.props["isOn"]?.joinToString("") ?: node.events["output"] ?: ""
            val checked = (state.vars[varName] ?: state.getVar(varName)).contains("true")

            Switch(
                checked = checked,
                enabled = isEnabled,
                onCheckedChange = {
                    state.setVar(varName, it.toString())
                },
                modifier = modifier
            )
        }
        "slider" -> {
            val varName = node.props["value"]?.joinToString("") ?: node.events["output"] ?: ""
            val min = engine.evalExpr(node.props["min"]).toFloatOrNull() ?: 0f
            val max = engine.evalExpr(node.props["max"]).toFloatOrNull() ?: 100f
            val value = (state.vars[varName] ?: state.getVar(varName)).toFloatOrNull() ?: min

            Slider(
                value = value,
                enabled = isEnabled,
                onValueChange = {
                    state.setVar(varName, it.toString())
                },
                valueRange = min..max,
                modifier = modifier.fillMaxWidth()
            )
        }
        "divider" -> {
            HorizontalDivider(modifier = modifier.padding(vertical = 8.dp))
        }
    }

    if (idVal.isNotEmpty()) {
        state.dynamicNodes[idVal]?.forEach { dNode ->
            WarpComponentRenderer(dNode, engine, state)
        }
    }
}

fun parseColor(colorStr: String): Color {
    return when (colorStr) {
        "yellow" -> Color(0xFFFBC02D)
        "red" -> Color(0xFFB3261E)
        "blue" -> Color(0xFF0A56D0)
        "gray" -> Color(0xFF808080)
        "black" -> Color(0xFF000000)
        "white" -> Color(0xFFFFFFFF)
        else -> if (colorStr.startsWith("#")) {
            try { Color(android.graphics.Color.parseColor(colorStr)) } catch (e: Exception) { Color.Unspecified }
        } else Color.Unspecified
    }
}
