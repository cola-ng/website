this is a complex task that requires a lot of work.

It contains backend and frontend.

For frontend, it use Solidjs and tailwindcss as frontend language.
I want to create a english learning website, just imagine it, you need to design professional and user-friendly interface you introduce.
The core features:

通过 AI 主动聊天的方式, 让用户敢于开口, 在与 AI 聊天的过程中, 可以大胆犯错, 而不感到害羞.
AI 主动引导用户聊天, 并在过程中记录用户的错误, 以及改进建议.
AI 还会根据这里信息, 自动生成复习计划, 并将计划带入后续聊天, 帮助用户在潜移默化中巩固学习.
AI 还可以模拟不同地区口音, 不同的音色, 让用户听力能力大增.
AI 会模拟场景, 给用户带入感.
AI 甚至会将某些电影等里面的经典对白带入到聊天之中, 让用户在聊天中感受到真实的场景.

Most of backend work is create a RESTful API. It use Rust as backend language.
It use Salvo web framework to create the API.
It use Pgsql databse.
It use diesel orm to interact with database.
For how to use diesel, please refer to its documentation: https://diesel.rs/.
Also you have a example project: D:\Repos\crates.io. Use same or familiar method to create db connection and connection pool.
For how to use salvo, please refer to its documentation: https://salvo.rs/ and it's repo: D:\Works\salvo-rs\salvo.


复杂任务, 请计划好, 再进行. 过程中可以把中间每一阶段的工作都单独提交到 git 仓库中.
检查现有的所有后端代码 crates/server. 里面有很多的错误, 因为是从其他项目拷贝过来的. 修复原则是尽量少删除代码, 让程序能够正常工作.
可以参考 main-beta-last.sql 表里面的相关信息, 在数据库缺失时新建相关数据库. 其中的 permissions 是实现相关权限管理的功能.

后端功能包括:
用户注册和登录
用户可以在网站上创建一个账号, 并登录到网站.
用户可以在网站上设置个人信息, 如姓名, 邮箱, 手机号等.
用户可以在网站上查看自己的学习记录, 如复习计划, 错误记录等.
用户可以通过第三方平台通过  oauth  或者 oidc  登录本网站.
第三方平台登录后, 前端页面 (front) 里面需要给出一个当前用户是否需要绑定现有用户的一个选项. 用户可以选择绑定或者跳过.
管理员有权限可以删除用户等.

我们还有一个桌面的系统, 所以需要后端协同前端提供一个从桌面系统登录到网站的接口. 用户登录成功后跳转到用户的桌面应用中.




你需要完成一个 rust 文件, 放在 crates/server/src/bin 下面, 一个 words.rs 文件, 负责生成单词数据.
字典的条目通过解析 D:\Works\words\简明英汉字典增强版(3407926条).txt 下的每一行, 得到单词.解析就是每一行的制表符之前的为单词.

具体实现: 你需要查看 bigmodel 相关技术文档, 了解具体接入的技术实现细节



字典的条目通过解析 D:\Works\words\简明英汉字典增强版(3407926条).txt 下的每一行, 得到单词.解析就是每一行的制表符之前的为单词.


你需要完成一个 rust 文件, 放在 crates/server/src/bin 下面, 一个 words.rs 文件, 负责生成字典数据.
遍历 跟目录下的 words-all.txt 文件,
请求 bigmodel 的 AI 的api, 通过让他成为一个专职的语言专家或者你手头有几部著名的英文大字典, 返回 一个 json 格式的字典数据.

字典数据格式参见  word_item.json.

文件按持续保持到 words 文件夹下面.