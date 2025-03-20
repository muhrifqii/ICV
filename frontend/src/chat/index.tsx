import { Button } from '@/components/ui/button'
import { ChatInput } from '@/components/ui/chat/chat-input'
import { ChatMessageList } from '@/components/ui/chat/chat-message-list'
import { ArrowUp, Paperclip } from 'lucide-react'
import { AnimatePresence, motion } from 'framer-motion'
import React, { useEffect, useRef } from 'react'
import { ChatBubble, ChatBubbleAction, ChatBubbleMessage } from '@/components/ui/chat/chat-bubble'
import { Avatar, AvatarImage } from '@/components/ui/avatar'
import logo from '/logo.svg'
import ChatActions from './config'

function Chat() {
  const messagesContainerRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const formRef = useRef<HTMLFormElement>(null)

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      handleSendMessage(e as unknown as React.FormEvent<HTMLFormElement>)
    }
  }
  const handleSendMessage = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault()
    if (!input) return

    setMessages(messages => [
      ...messages,
      {
        id: messages.length + 1,
        role: 'user',
        message: input
      }
    ])

    setInput('')
    formRef.current?.reset()
  }

  const getMessageVariant = (role: string) => (role === 'ai' ? 'received' : 'sent')

  useEffect(() => {
    if (inputRef.current) {
      inputRef.current.focus()
    }
  }, [])

  useEffect(() => {
    if (messagesContainerRef.current) {
      messagesContainerRef.current.scrollTop = messagesContainerRef.current.scrollHeight
    }
  }, [messages])

  return (
    <div className='flex flex-col size-full'>
      <div className='flex-1 w-full overflow-y-auto bg-muted/40'>
        <ChatMessageList ref={messagesContainerRef}>
          {/* Chat messages */}
          <AnimatePresence>
            {messages.map((message, index) => {
              const variant = getMessageVariant(message.role!)
              return (
                <motion.div
                  key={index}
                  layout
                  initial={{ opacity: 0, scale: 1, y: 50, x: 0 }}
                  animate={{ opacity: 1, scale: 1, y: 0, x: 0 }}
                  exit={{ opacity: 0, scale: 1, y: 1, x: 0 }}
                  transition={{
                    opacity: { duration: 0.1 },
                    layout: {
                      type: 'spring',
                      bounce: 0.3,
                      duration: index * 0.05 + 0.2
                    }
                  }}
                  style={{ originX: 0.5, originY: 0.5 }}
                  className='flex flex-col gap-2 p-4'
                >
                  <ChatBubble key={index} variant={variant}>
                    {message.role === 'ai' && (
                      <Avatar>
                        <AvatarImage src={logo} alt='Avatar' />
                      </Avatar>
                    )}

                    <ChatBubbleMessage isLoading={message.isLoading}>
                      {message.message}
                      {message.role === 'ai' && (
                        <div className='flex items-center mt-1.5 gap-1'>
                          {!message.isLoading && (
                            <>
                              {ChatActions.map((icon, index) => {
                                const Icon = icon.icon
                                return (
                                  <ChatBubbleAction
                                    variant='outline'
                                    className='size-6'
                                    key={index}
                                    icon={<Icon className='size-3' />}
                                    onClick={() =>
                                      console.log('Action ' + icon.label + ' clicked for message ' + index)
                                    }
                                  />
                                )
                              })}
                            </>
                          )}
                        </div>
                      )}
                    </ChatBubbleMessage>
                  </ChatBubble>
                </motion.div>
              )
            })}
          </AnimatePresence>
        </ChatMessageList>
      </div>
      <div className='px-4 pb-4 bg-muted/40'>
        <form
          ref={formRef}
          onSubmit={handleSendMessage}
          className='relative rounded-lg border bg-background focus-within:ring-1 focus-within:ring-ring'
        >
          <ChatInput
            ref={inputRef}
            onKeyDown={handleKeyDown}
            onChange={handleInputChange}
            placeholder='Type your message here...'
            className='min-h-12 resize-none rounded-lg bg-background border-0 p-3 shadow-none focus-visible:ring-0'
          />
          <div className='flex items-center p-3 pt-0'>
            <Button variant='ghost' size='icon'>
              <Paperclip className='size-4' />
              <span className='sr-only'>Attach file</span>
            </Button>

            <Button disabled={!input || isLoading} type='submit' size='sm' className='ml-auto gap-1.5'>
              Send Message
              <ArrowUp className='size-3.5' />
            </Button>
          </div>
        </form>
      </div>
    </div>
  )
}

export default Chat
