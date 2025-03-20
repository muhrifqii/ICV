import { ThemeProvider } from '@/components/theme-provider'
import Chat from './chat'

export default function App() {
  return (
    <ThemeProvider defaultTheme='dark' storageKey='vite-ui-theme'>
      <Chat />
    </ThemeProvider>
  )
}
