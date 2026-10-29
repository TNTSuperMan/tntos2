# tntos2
適当に付けた名前です

MikanOSを参考にしながらRustでOS作ります

## progress
ブートローダーは大体完成(MikanOSのブートローダーと同等の感じ)  
カーネルは全然できてません  
パニックハンドラとちょっとした画面描画ぐらいはあります

## 環境構築とか
まずRust入れてください  
MSRVは知りませんとりあえず1.99.1使ってます  
入れないといけないターゲットは以下です`rustup target add ...`で追加
- x86_64-unknown-uefi
- x86_64-unknown-none

次に[Bun](https://bun.com)入れてください  
`bun scripts/bdfload.ts`でフォントバイナリを作ります

そしてqemu入れてください  
詳しく書くのはめんどいので省略ですが[uefi-rsチュートリアルのとこ](https://rust-osdev.github.io/uefi-rs/tutorial/vm.html)とか参考になると思います

vscodeの設定ファイルは無いです、zedの設定ファイルだけ入ってます

## 著作権表記とか
- 参考: [uchan-nos/mikanos](https://github.com/uchan-nos/MikanOS) - Apache-2.0ライセンス
- 利用ライブラリ: [rust-osdev/uefi-rs](https://github.com/rust-osdev/uefi-rs) - MITライセンス / Apache-2.0ライセンス
- フォント: [東雲フォント](http://openlab.ring.gr.jp/efont/shinonome/) - Public Domain
