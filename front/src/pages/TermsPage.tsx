import { Footer } from '../components/Footer'
import { Header } from '../components/Header'

export function TermsPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />
      <main className="mx-auto max-w-4xl p-4">
        <div className="bg-white rounded-xl shadow-lg p-8">
          <h1 className="text-2xl font-bold text-gray-900 mb-2">服务条款</h1>
          <p className="text-sm text-gray-500 mb-8">
            生效日期：2025年1月1日 | 最后更新：2025年1月1日
          </p>

          <div className="prose prose-sm max-w-none text-gray-700 space-y-6">
            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">一、总则</h2>
              <p>
                欢迎使用开朗英语（以下简称"本服务"或"本平台"）。本服务条款（以下简称"本条款"）是您与开朗英语运营方（以下简称"我们"）之间关于使用本服务所订立的协议。
              </p>
              <p>
                <strong>请您在使用本服务前，仔细阅读并充分理解本条款的全部内容。</strong>如果您不同意本条款的任何内容，请勿注册或使用本服务。您注册、登录、使用本服务的行为即视为您已阅读、理解并同意接受本条款的约束。
              </p>
              <p>
                本条款构成您与我们之间具有法律约束力的协议。我们有权根据法律法规的变化、业务发展需要等原因修改本条款，修改后的条款将在本页面公布。若您在条款修改后继续使用本服务，即视为您接受修改后的条款。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">二、服务说明</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">2.1 服务内容</h3>
              <p>开朗英语是一款基于人工智能技术的英语学习平台，为用户提供以下服务：</p>
              <ul className="list-disc pl-5 space-y-1">
                <li>AI智能对话练习：与AI进行实时英语对话，练习口语表达能力</li>
                <li>场景模拟训练：提供多种真实场景的英语对话练习</li>
                <li>跟读训练：AI智能评分，帮助纠正发音</li>
                <li>词汇学习与复习：科学的间隔重复系统，巩固词汇记忆</li>
                <li>学习数据分析：追踪学习进度，提供个性化学习建议</li>
                <li>其他我们不时推出的学习功能</li>
              </ul>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">2.2 服务形式</h3>
              <p>
                本服务通过网页端、移动应用程序（如适用）及其他我们授权的方式提供。我们保留随时修改、暂停或终止部分或全部服务的权利，无需事先通知用户（法律另有规定的除外）。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">2.3 服务限制</h3>
              <p>
                本服务中的AI生成内容仅供学习参考，不构成专业的语言教学、翻译或其他专业建议。用户应自行判断AI生成内容的准确性和适用性。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">三、账户注册与管理</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">3.1 注册资格</h3>
              <p>您确认，在您完成注册程序时，您应具备中华人民共和国法律规定的完全民事行为能力。若您不具备前述民事行为能力，则应在法定监护人的监护下完成注册，并在使用本服务时取得监护人的同意。</p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">3.2 账户信息</h3>
              <ul className="list-disc pl-5 space-y-1">
                <li>您承诺在注册时提供真实、准确、完整的信息，并及时更新。</li>
                <li>您应妥善保管账户信息和密码，对账户下的所有行为承担责任。</li>
                <li>如发现账户被未经授权使用或存在安全漏洞，请立即通知我们。</li>
                <li>您不得将账户转让、出租、出借给他人使用。</li>
              </ul>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">3.3 账户安全</h3>
              <p>
                因您自身原因导致的账户密码泄露、账户被盗用等情形，由您自行承担相应责任和损失。我们不对因第三方行为或您自身过错导致的任何损失承担责任。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">四、用户行为规范</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">4.1 合法使用</h3>
              <p>您在使用本服务时，应遵守中华人民共和国相关法律法规及本条款的规定。您不得利用本服务从事任何违法违规活动。</p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">4.2 禁止行为</h3>
              <p>您在使用本服务时，不得有以下行为：</p>
              <ul className="list-disc pl-5 space-y-1">
                <li>发布、传播违反国家法律法规的内容</li>
                <li>发布、传播侵犯他人知识产权、商业秘密、隐私权等合法权益的内容</li>
                <li>发布、传播虚假信息、垃圾信息或含有恶意代码的内容</li>
                <li>发布、传播淫秽、色情、暴力、恐怖或教唆犯罪的内容</li>
                <li>发布、传播歧视、侮辱、诽谤他人的内容</li>
                <li>干扰、破坏本服务的正常运行</li>
                <li>利用技术手段非法获取本服务的数据或内容</li>
                <li>未经授权使用自动化工具批量访问本服务</li>
                <li>从事任何可能损害我们或其他用户权益的行为</li>
                <li>其他我们认为不适当的行为</li>
              </ul>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">4.3 违规处理</h3>
              <p>
                如您违反本条款或相关法律法规，我们有权采取警告、限制功能、暂停服务、终止账户等措施，并保留追究法律责任的权利。因您的违规行为导致我们或第三方损失的，您应承担全部赔偿责任。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">五、知识产权</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">5.1 我们的权利</h3>
              <p>
                本服务的所有内容，包括但不限于文字、图片、音频、视频、软件、程序、数据、界面设计、版面框架等，其知识产权均归我们或相关权利人所有，受中华人民共和国著作权法、商标法、专利法及其他相关法律法规的保护。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">5.2 用户内容</h3>
              <p>
                您在使用本服务过程中上传、发布的内容（包括但不限于文字、语音、图片等），您保证拥有相应的合法权利。您授予我们非独占的、免费的、永久的、可转授权的许可，使我们可以在全球范围内使用、复制、修改、改编、发布、翻译、创作衍生作品、传播、表演和展示此等内容，用于改进服务、开发新功能等目的。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">5.3 禁止侵权</h3>
              <p>
                未经我们或相关权利人书面许可，您不得复制、修改、发布、传播、展示或以其他方式使用本服务中的任何内容。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">六、付费服务（如适用）</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">6.1 收费说明</h3>
              <p>
                本服务的部分功能可能需要付费使用。具体收费标准以服务页面展示为准。我们有权根据市场情况调整收费标准，调整后的标准将在服务页面公布。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">6.2 支付与退款</h3>
              <ul className="list-disc pl-5 space-y-1">
                <li>您应通过我们指定的支付方式完成付款</li>
                <li>付费服务一经购买，除法律规定或本条款另有约定外，一般不予退款</li>
                <li>如因我们原因导致服务无法正常提供，您可申请相应退款</li>
                <li>虚拟商品（如会员权益、学习资源等）的退款政策以具体产品说明为准</li>
              </ul>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">6.3 订阅服务</h3>
              <p>
                如您订阅了自动续费服务，在订阅期满前您未主动取消的，我们将自动为您续费。您可随时在账户设置中取消自动续费。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">七、免责声明</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">7.1 服务"按现状"提供</h3>
              <p>
                本服务按"现状"和"可用"基础提供，我们不对服务的及时性、安全性、准确性、完整性作出任何明示或暗示的保证。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">7.2 AI内容免责</h3>
              <p>
                本服务中的AI生成内容可能存在错误或不准确之处。我们不对AI生成内容的准确性、完整性、适用性承担任何责任。用户应自行判断和核实AI生成的学习内容。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">7.3 第三方链接</h3>
              <p>
                本服务可能包含指向第三方网站或服务的链接。我们对第三方网站或服务的内容、隐私政策或做法不承担任何责任。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">7.4 不可抗力</h3>
              <p>
                因不可抗力（包括但不限于自然灾害、政府行为、战争、罢工、网络攻击、系统故障等）导致服务中断或无法正常提供的，我们不承担责任，但将尽合理努力尽快恢复服务。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">八、责任限制</h2>
              <p>
                <strong>在法律允许的最大范围内</strong>，我们对因使用或无法使用本服务而产生的任何直接、间接、附带、特殊、惩罚性或后果性损害（包括但不限于利润损失、数据丢失、商誉损害等）不承担责任，无论该等损害是否可预见，也无论我们是否已被告知该等损害的可能性。
              </p>
              <p>
                如因我们的过错导致您遭受损失，我们的赔偿责任以您在损失发生前12个月内向我们支付的服务费用为限（如有）。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">九、服务变更与终止</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">9.1 服务变更</h3>
              <p>
                我们保留随时修改、暂停或终止本服务（或其任何部分）的权利，无需事先通知（法律另有规定的除外）。对于服务的修改、暂停或终止，我们对您或任何第三方不承担责任。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">9.2 账户终止</h3>
              <p>发生以下情形时，我们有权终止向您提供服务并注销您的账户：</p>
              <ul className="list-disc pl-5 space-y-1">
                <li>您违反本条款或相关法律法规</li>
                <li>您的账户连续12个月以上未登录使用</li>
                <li>法律法规或监管要求</li>
                <li>其他我们认为需要终止服务的情形</li>
              </ul>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">9.3 终止后果</h3>
              <p>
                账户终止后，您将无法继续使用本服务，您账户中的数据可能被删除（法律要求保留的除外）。已购买的付费服务中未使用的部分一般不予退还。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">十、争议解决</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">10.1 适用法律</h3>
              <p>
                本条款的订立、效力、解释、履行和争议解决均适用中华人民共和国法律（不包括其冲突法规则）。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">10.2 争议解决方式</h3>
              <p>
                因本条款或本服务引起的或与之相关的任何争议，双方应首先通过友好协商解决。协商不成的，任何一方均可将争议提交至我们所在地有管辖权的人民法院诉讼解决。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">十一、其他条款</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">11.1 完整协议</h3>
              <p>
                本条款（包括我们可能发布的隐私政策及其他规则）构成您与我们之间关于本服务的完整协议，取代您与我们之间就本服务达成的所有先前或同期书面或口头协议。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">11.2 可分割性</h3>
              <p>
                如本条款的任何条款被认定为无效或不可执行，该条款应在最小必要范围内进行修改以使其有效和可执行，其余条款仍完全有效。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">11.3 权利不放弃</h3>
              <p>
                我们未能行使或延迟行使本条款项下的任何权利，不构成对该权利的放弃，也不影响我们将来行使该权利。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">11.4 通知</h3>
              <p>
                我们向您发送的通知可以通过以下方式之一进行：站内消息、电子邮件、短信、公告或其他合理方式。该等通知在发送时即视为已送达。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">十二、联系我们</h2>
              <p>如果您对本服务条款有任何疑问，请通过以下方式与我们联系：</p>
              <ul className="list-none pl-0 mt-2 space-y-1">
                <li>电子邮箱：support@kailang.ai</li>
                <li>客服热线：400-XXX-XXXX</li>
                <li>通讯地址：[公司注册地址]</li>
              </ul>
            </section>
          </div>
        </div>
      </main>
      <Footer />
    </div>
  )
}
