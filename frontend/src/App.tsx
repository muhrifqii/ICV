import { useState, use } from 'react'
import { ThemeProvider } from '@/components/theme-provider'

export default function App() {
  return (
    <ThemeProvider defaultTheme='dark' storageKey='vite-ui-theme'>
      <></>
    </ThemeProvider>
  )
}

function AskUI() {
  const [messages, setMessages] = useState([
    { id: 1, text: '🧑‍💻 Hello! How can I assist you with your tech career today?', sender: 'bot' }
  ])
  const [input, setInput] = useState('')

  const sendMessage = () => {
    if (!input.trim()) return
    const newMessage = { id: messages.length + 1, text: input, sender: 'user' }
    setMessages([...messages, newMessage])
    setInput('')
  }

  return (
    <div className='min-h-screen flex flex-col items-center justify-center bg-gray-900 text-white p-6'>
      <Card className='w-full max-w-2xl bg-gray-800 p-4 rounded-lg shadow-lg flex flex-col h-[70vh]'>
        <ScrollArea className='flex-1 p-3 space-y-4 overflow-y-auto'>
          {messages.map(msg => (
            <div
              key={msg.id}
              className={`p-3 rounded-lg max-w-xs ${msg.sender === 'user' ? 'bg-blue-600 self-end' : 'bg-gray-700 self-start'}`}
            >
              <p className='text-sm'>{msg.text}</p>
            </div>
          ))}
        </ScrollArea>

        <div className='flex items-center gap-3 p-3 border-t border-gray-700'>
          <Input
            type='text'
            placeholder='Ask a career-related question...'
            className='flex-1 bg-gray-700 text-white'
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyPress={e => e.key === 'Enter' && sendMessage()}
          />
          <Button className='bg-blue-600' onClick={sendMessage}>
            Send
          </Button>
        </div>
      </Card>
    </div>
  )
}
