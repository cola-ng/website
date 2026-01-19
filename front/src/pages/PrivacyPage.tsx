import { Footer } from '../components/Footer'
import { Header } from '../components/Header'

export function PrivacyPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />
      <main className="mx-auto max-w-6xl p-4">
        <div className="bg-white rounded-xl shadow-lg p-8">
          <h1 className="text-2xl font-bold text-gray-900 mb-2">隐私政策</h1>
          <p className="text-sm text-gray-500 mb-8">
            生效日期：2025年1月1日 | 最后更新：2025年1月1日
          </p>

          <div className="prose prose-sm max-w-none text-gray-700 space-y-6">
            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">一、引言</h2>
              <p>
                开朗英语（以下简称"我们"或"本平台"）深知个人信息对您的重要性，并会尽全力保护您的个人信息安全可靠。我们致力于维持您对我们的信任，恪守以下原则，保护您的个人信息：权责一致原则、目的明确原则、选择同意原则、最少够用原则、确保安全原则、主体参与原则、公开透明原则等。
              </p>
              <p>
                本隐私政策适用于您通过网页端、移动应用程序及其他方式访问和使用开朗英语服务时我们收集、使用、存储和共享您个人信息的行为。请您在使用我们的服务前，仔细阅读并充分理解本政策的全部内容。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">二、我们收集的信息</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">2.1 您主动提供的信息</h3>
              <ul className="list-disc pl-5 space-y-1">
                <li><strong>账户信息：</strong>当您注册账户时，我们会收集您的电子邮箱地址、用户名、密码（加密存储）及手机号码（可选）。</li>
                <li><strong>个人资料：</strong>您可选择提供的昵称、头像、学习目标、英语水平等信息。</li>
                <li><strong>学习内容：</strong>您在使用服务过程中产生的对话记录、语音输入、学习笔记等。</li>
                <li><strong>反馈信息：</strong>您向我们提交的意见反馈、问题报告等。</li>
              </ul>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">2.2 自动收集的信息</h3>
              <ul className="list-disc pl-5 space-y-1">
                <li><strong>设备信息：</strong>设备型号、操作系统版本、唯一设备标识符、浏览器类型等。</li>
                <li><strong>日志信息：</strong>访问时间、访问页面、IP地址、使用时长等。</li>
                <li><strong>学习数据：</strong>学习进度、练习成绩、学习时长统计、词汇掌握情况等。</li>
              </ul>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">2.3 第三方来源的信息</h3>
              <p>
                当您选择使用第三方账号（如微信、Google等）登录时，我们会根据您的授权从第三方获取您的公开信息（如昵称、头像）。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">三、信息的使用</h2>
              <p>我们收集您的信息用于以下目的：</p>
              <ul className="list-disc pl-5 space-y-1">
                <li><strong>提供服务：</strong>为您提供AI英语对话练习、场景模拟、跟读训练、词汇复习等核心功能。</li>
                <li><strong>个性化体验：</strong>根据您的学习数据和偏好，提供个性化的学习建议和内容推荐。</li>
                <li><strong>服务改进：</strong>分析使用数据以改进产品功能、优化用户体验。</li>
                <li><strong>安全保障：</strong>验证身份、预防欺诈、保障账户安全。</li>
                <li><strong>客户服务：</strong>响应您的咨询、投诉和建议。</li>
                <li><strong>法律合规：</strong>遵守适用法律法规的要求。</li>
              </ul>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">四、信息的存储与保护</h2>
              <h3 className="font-medium text-gray-800 mt-4 mb-2">4.1 存储地点</h3>
              <p>
                您的个人信息将存储于中华人民共和国境内的服务器。如需跨境传输，我们将严格遵守相关法律法规，并采取必要的安全措施。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">4.2 存储期限</h3>
              <p>
                我们仅在为实现本政策所述目的所必需的期限内保留您的个人信息，除非法律要求或允许更长的保留期限。账户注销后，我们将在合理期限内删除或匿名化处理您的个人信息。
              </p>

              <h3 className="font-medium text-gray-800 mt-4 mb-2">4.3 安全措施</h3>
              <p>我们采取以下措施保护您的信息安全：</p>
              <ul className="list-disc pl-5 space-y-1">
                <li>使用SSL/TLS加密传输敏感数据</li>
                <li>对密码等敏感信息进行加密存储</li>
                <li>实施访问控制，限制对个人信息的访问权限</li>
                <li>定期进行安全审计和漏洞扫描</li>
                <li>制定数据安全事件应急响应计划</li>
              </ul>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">五、信息的共享与披露</h2>
              <p>我们承诺不会出售您的个人信息。仅在以下情况下，我们可能会共享您的信息：</p>
              <ul className="list-disc pl-5 space-y-1">
                <li><strong>获得您的明确同意：</strong>在获得您的明确同意后，我们可能与第三方共享您的信息。</li>
                <li><strong>服务提供商：</strong>我们可能委托授权合作伙伴提供技术基础设施、数据分析等服务，这些合作伙伴须遵守严格的数据保护义务。</li>
                <li><strong>法律要求：</strong>根据法律法规的要求、司法程序、诉讼和/或公共机构和政府部门的要求。</li>
                <li><strong>保护权益：</strong>为保护我们、用户或公众的权利、财产或安全所必需。</li>
                <li><strong>业务转让：</strong>如发生合并、收购或资产出售，您的个人信息可能作为交易资产转让，我们将确保受让方继续遵守本隐私政策。</li>
              </ul>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">六、Cookie及类似技术</h2>
              <p>
                我们使用Cookie和类似技术来提供、保护和改进我们的服务，例如记住您的登录状态、分析服务使用情况等。您可以通过浏览器设置管理Cookie，但禁用Cookie可能影响部分功能的正常使用。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">七、您的权利</h2>
              <p>根据适用法律，您对您的个人信息享有以下权利：</p>
              <ul className="list-disc pl-5 space-y-1">
                <li><strong>访问权：</strong>您有权访问我们持有的您的个人信息。</li>
                <li><strong>更正权：</strong>您有权要求更正不准确或不完整的个人信息。</li>
                <li><strong>删除权：</strong>在特定情况下，您有权要求删除您的个人信息。</li>
                <li><strong>撤回同意：</strong>您有权随时撤回您之前给予的同意。</li>
                <li><strong>账户注销：</strong>您可以通过账户设置或联系我们申请注销账户。</li>
                <li><strong>数据可携带：</strong>您有权获取您提供给我们的个人信息的副本。</li>
              </ul>
              <p className="mt-2">
                如需行使上述权利，请通过本政策末尾的联系方式与我们联系。我们将在验证您的身份后，在法定期限内响应您的请求。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">八、未成年人保护</h2>
              <p>
                我们的服务主要面向成年人。若您是未满18周岁的未成年人，请在法定监护人的陪同和指导下阅读本政策，并在取得法定监护人的同意后使用我们的服务。
              </p>
              <p>
                如果我们发现在未获得可证实的监护人同意的情况下收集了未成年人的个人信息，我们将尽快删除相关数据。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">九、政策更新</h2>
              <p>
                我们可能会不时更新本隐私政策。更新后的政策将在本页面发布，并注明生效日期。对于重大变更，我们将通过站内通知、电子邮件或其他适当方式通知您。
              </p>
              <p>
                若您在政策更新后继续使用我们的服务，即表示您同意受更新后的政策约束。
              </p>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-gray-900 mb-3">十、联系我们</h2>
              <p>如果您对本隐私政策有任何疑问、意见或建议，请通过以下方式与我们联系：</p>
              <ul className="list-none pl-0 mt-2 space-y-1">
                <li>电子邮箱：privacy@kailang.ai</li>
                <li>客服热线：400-XXX-XXXX</li>
                <li>通讯地址：[公司注册地址]</li>
              </ul>
              <p className="mt-2">
                我们将在收到您的问题后尽快回复，一般不超过15个工作日。
              </p>
            </section>
          </div>
        </div>
      </main>
      <Footer />
    </div>
  )
}
