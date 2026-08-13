# warp-mirae
#### 기본

```これらは同じとなる

varname.$+[
 1
 2
]

↓=↑

varname.$+1

varname.$+2

```


#### calc

varname.1 //varname変数に1を代入

varname.$+1 //varname変数に1を加算して代入

varname.$+"1" //varname変数に文字列"1"を結合

varname.varname1+1 //varname変数に変数varname1の値に1加算して代入



varname.ran(1,10) //varname変数に1から10までのランダムな整数を代入



#### get

varname.get(2:6,varname1) //varname変数に変数varname1の2番目から6番目までの文字列を代入

varname.long("aaaa") //varname変数に文字列"aaaa"の長さを代入




#### list

listname.add("a","b","c")//リストlistnameにa,b,cを追加

listname.add(varname1,varname2,varname3)//リストlistnameにvarname1,varname2,varname3の内容を追加

listname.set(1,"first")//リストlistnameの1番目の要素をfirstに設定

varname."内容:"+list(listname,4) //変数varnameに文字列"内容:"とlistnameの4番目の要素を結合して代入

listname.delete(1) //リストlistnameの1番目の要素を削除

varname.long(listname) //変数varnameにリストlistnameの長さを代入

varname.get(listname,1:3) //変数varnameにリストlistnameの1番目から3番目までの要素を代入
varname.get(listname,1) //変数varnameにリストlistnameの1番目の要素を代入
varname.get(listname,"a") //変数varnameにリストlistnameの"a"が含まれる場所の番号(複数個の場合一番最初)



#### sys

wait(1ms) //1ms待機

list(listname) //リストlistnameを開始
deleteList(listname) //リストlistnameを削除

var(varname) //変数varnameを開始
deleteVar(varname) //変数varnameを削除

varOrListname.replace("a","b") //変数varOrListnameの"a"を"b"に置換(変数とリスト両方で利用可能だが，リストの場合は内容が完全一致の場合のみ)

upperMode(false) //文字列，変数，リストで大文字を区別しない
upperMode(true) //文字列，変数，リストで大文字を区別する

varname.null //変数varnameの内容を空にする

varname."hello\nworld" //"\n"は改行を示す



#### if
if(varname=="a"){} //変数varnameが"a"の場合

if(varname>10){} //変数varnameが10より大きい場合

if(varname<10){} //変数varnameが10より小さい場合

if(varname>=10){} //変数varnameが10以上の場合

if(varname<=10){} //変数varnameが10以下の場合

if(varname!=10){} //変数varnameが10でない場合


if(varname=="a"){

}else if(varname=="b"){

}else{

} //変数varnameが"a"の場合、"b"の場合、それ以外の場合

if(listname.includes("a")){} //リストlistnameに"a"という要素が含まれる場合

if(varname.includes("a")){} //変数varnameに"a"という文字列が含まれる場合



#### block

block(blockname){
    listname.add(1,2,3)
}
//ブロックblocknameを開始

blockname //ブロックblocknameの内容を実行

#### loop

loop(10){
    listname.add(1,2,3)
} //リストlistnameに1,2,3を10回追加

loop(long("aa")){
    listname.add(1,2,3)
} //リストlistnameに1,2,3を2回追加

loop(ran(1,10)){
    listname.add(1,2,3)
} //リストlistnameに1,2,3を1から10までのランダムな整数回追加


loop(varname=="10"){
    varname.$+1
} //変数varnameが10になるまで、変数varnameに1を加算して代入
