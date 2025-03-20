export type Roles = Partial<{
  user: unknown
  system: unknown
  assistant: unknown
}>

export type Message = {
  content: string
  timestamp: string
  role: Roles
  stared: boolean
  isLoading?: boolean
}
