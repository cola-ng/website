import { useState } from 'react'
import { RotateCcw, CheckCircle, XCircle, TrendingUp, Clock, Target, ChevronRight } from 'lucide-react'

import { Footer } from '../components/Footer'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs'
import { useAuth } from '../lib/auth'
import { cn } from '../lib/utils'

interface ReviewWord {
  id: string
  word: string
  meaning: string
  example: string
  lastReview: string
  mastery: number
  dueIn?: string
}

const dueWords: ReviewWord[] = [
  {
    id: '1',
    word: 'accommodation',
    meaning: '住所，住宿',
    example: 'We need to find accommodation for the night.',
    lastReview: '2 days ago',
    mastery: 60,
    dueIn: 'Now',
  },
  {
    id: '2',
    word: 'itinerary',
    meaning: '行程表',
    example: "What's our itinerary for tomorrow?",
    lastReview: '3 days ago',
    mastery: 45,
    dueIn: 'Now',
  },
  {
    id: '3',
    word: 'reservation',
    meaning: '预订',
    example: 'I have a reservation under the name Smith.',
    lastReview: '1 week ago',
    mastery: 70,
    dueIn: '2 hours',
  },
]

const mistakes: ReviewWord[] = [
  {
    id: '4',
    word: 'affect vs effect',
    meaning: 'affect (v.) 影响 / effect (n.) 效果',
    example: 'The weather affects my mood. The effect was immediate.',
    lastReview: '1 day ago',
    mastery: 30,
  },
  {
    id: '5',
    word: 'their vs there',
    meaning: 'their (他们的) / there (那里)',
    example: 'Their car is over there.',
    lastReview: '2 days ago',
    mastery: 40,
  },
]

const masteredWords: ReviewWord[] = [
  {
    id: '6',
    word: 'schedule',
    meaning: '日程安排',
    example: "Let me check my schedule.",
    lastReview: '2 weeks ago',
    mastery: 95,
  },
  {
    id: '7',
    word: 'appointment',
    meaning: '预约',
    example: 'I have an appointment at 3pm.',
    lastReview: '1 week ago',
    mastery: 90,
  },
]

function WordCard({ word, showDue = false }: { word: ReviewWord; showDue?: boolean }) {
  const [flipped, setFlipped] = useState(false)

  return (
    <div
      onClick={() => setFlipped(!flipped)}
      className="bg-white border rounded-xl p-4 cursor-pointer hover:shadow-md transition-all"
    >
      {!flipped ? (
        <div>
          <div className="flex items-start justify-between">
            <h3 className="text-lg font-semibold text-gray-900">{word.word}</h3>
            {showDue && word.dueIn && (
              <span className={cn(
                'text-xs px-2 py-0.5 rounded-full',
                word.dueIn === 'Now' ? 'bg-red-100 text-red-600' : 'bg-amber-100 text-amber-600'
              )}>
                {word.dueIn}
              </span>
            )}
          </div>
          <p className="text-sm text-gray-500 mt-1">点击查看释义</p>
          <div className="mt-3 flex items-center gap-2">
            <div className="flex-1 h-2 bg-gray-200 rounded-full">
              <div
                className={cn(
                  'h-2 rounded-full',
                  word.mastery >= 80 ? 'bg-green-500' : word.mastery >= 50 ? 'bg-amber-500' : 'bg-red-500'
                )}
                style={{ width: `${word.mastery}%` }}
              />
            </div>
            <span className="text-xs text-gray-500">{word.mastery}%</span>
          </div>
        </div>
      ) : (
        <div>
          <h3 className="text-lg font-semibold text-gray-900">{word.word}</h3>
          <p className="text-orange-600 font-medium mt-2">{word.meaning}</p>
          <p className="text-sm text-gray-600 mt-2 italic">"{word.example}"</p>
          <p className="text-xs text-gray-400 mt-2">上次复习: {word.lastReview}</p>
        </div>
      )}
    </div>
  )
}

function StatsPanel() {
  return (
    <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
      <div className="bg-orange-50 rounded-lg p-4 text-center">
        <div className="text-2xl font-bold text-orange-600">23</div>
        <div className="text-xs text-gray-500">待复习</div>
      </div>
      <div className="bg-green-50 rounded-lg p-4 text-center">
        <div className="text-2xl font-bold text-green-600">156</div>
        <div className="text-xs text-gray-500">已掌握</div>
      </div>
      <div className="bg-red-50 rounded-lg p-4 text-center">
        <div className="text-2xl font-bold text-red-600">8</div>
        <div className="text-xs text-gray-500">易错点</div>
      </div>
      <div className="bg-blue-50 rounded-lg p-4 text-center">
        <div className="text-2xl font-bold text-blue-600">85%</div>
        <div className="text-xs text-gray-500">正确率</div>
      </div>
    </div>
  )
}

export function ReviewPage() {
  const { token } = useAuth()

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto max-w-4xl p-4">
          <div className="bg-white rounded-xl shadow-lg p-8 text-center">
            <div className="text-6xl mb-4">📚</div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">复习巩固</h1>
            <p className="text-gray-600 mb-6">
              科学的间隔重复系统，帮助你巩固所学知识
            </p>
            <Button asChild>
              <a href="/login?redirectTo=/review">登录开始复习</a>
            </Button>
          </div>
        </div>
        <Footer />
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />

      <main className="mx-auto max-w-4xl p-4">
        <div className="bg-white rounded-xl shadow-lg p-6 mb-4">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h1 className="text-xl font-semibold text-gray-900">复习巩固</h1>
              <p className="text-sm text-gray-500">科学复习，牢记所学</p>
            </div>
            <Button>
              <RotateCcw className="h-4 w-4 mr-2" />
              开始复习
            </Button>
          </div>
          <StatsPanel />
        </div>

        <div className="bg-white rounded-xl shadow-lg overflow-hidden">
          <Tabs defaultValue="due">
            <div className="border-b px-6 py-3">
              <TabsList>
                <TabsTrigger value="due" className="gap-2">
                  <Clock className="h-4 w-4" />
                  待复习
                </TabsTrigger>
                <TabsTrigger value="mistakes" className="gap-2">
                  <XCircle className="h-4 w-4" />
                  易错点
                </TabsTrigger>
                <TabsTrigger value="mastered" className="gap-2">
                  <CheckCircle className="h-4 w-4" />
                  已掌握
                </TabsTrigger>
                <TabsTrigger value="stats" className="gap-2">
                  <TrendingUp className="h-4 w-4" />
                  统计
                </TabsTrigger>
              </TabsList>
            </div>

            <TabsContent value="due" className="p-6">
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {dueWords.map((word) => (
                  <WordCard key={word.id} word={word} showDue />
                ))}
              </div>
              {dueWords.length === 0 && (
                <div className="text-center py-8 text-gray-500">
                  太棒了！暂时没有需要复习的内容
                </div>
              )}
            </TabsContent>

            <TabsContent value="mistakes" className="p-6">
              <div className="mb-4 bg-amber-50 rounded-lg p-4">
                <p className="text-sm text-amber-800">
                  <Target className="h-4 w-4 inline mr-2" />
                  这些是你经常出错的词汇，多加练习可以帮助你克服这些难点
                </p>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {mistakes.map((word) => (
                  <WordCard key={word.id} word={word} />
                ))}
              </div>
            </TabsContent>

            <TabsContent value="mastered" className="p-6">
              <div className="mb-4 bg-green-50 rounded-lg p-4">
                <p className="text-sm text-green-800">
                  <CheckCircle className="h-4 w-4 inline mr-2" />
                  这些词汇你已经掌握得很好了，继续保持！
                </p>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                {masteredWords.map((word) => (
                  <WordCard key={word.id} word={word} />
                ))}
              </div>
            </TabsContent>

            <TabsContent value="stats" className="p-6">
              <div className="space-y-6">
                <div>
                  <h3 className="font-medium text-gray-900 mb-3">学习概览</h3>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="bg-gray-50 rounded-lg p-4">
                      <div className="text-sm text-gray-500">总学习词汇</div>
                      <div className="text-2xl font-bold text-gray-900">187</div>
                    </div>
                    <div className="bg-gray-50 rounded-lg p-4">
                      <div className="text-sm text-gray-500">学习天数</div>
                      <div className="text-2xl font-bold text-gray-900">32</div>
                    </div>
                    <div className="bg-gray-50 rounded-lg p-4">
                      <div className="text-sm text-gray-500">复习次数</div>
                      <div className="text-2xl font-bold text-gray-900">456</div>
                    </div>
                    <div className="bg-gray-50 rounded-lg p-4">
                      <div className="text-sm text-gray-500">平均掌握度</div>
                      <div className="text-2xl font-bold text-gray-900">78%</div>
                    </div>
                  </div>
                </div>

                <div>
                  <h3 className="font-medium text-gray-900 mb-3">本周复习</h3>
                  <div className="flex items-end gap-2 h-32">
                    {['一', '二', '三', '四', '五', '六', '日'].map((day, i) => (
                      <div key={day} className="flex-1 flex flex-col items-center gap-1">
                        <div
                          className="w-full bg-orange-400 rounded-t"
                          style={{ height: `${[60, 80, 45, 90, 70, 30, 50][i]}%` }}
                        />
                        <span className="text-xs text-gray-500">{day}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </TabsContent>
          </Tabs>
        </div>
      </main>
      <Footer />
    </div>
  )
}
