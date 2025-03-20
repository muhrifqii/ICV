import { create } from 'zustand'

interface State {
  draftInput: string
}

interface Action {
  handleInputChanges: (e: React.ChangeEvent<HTMLInputElement> | React.ChangeEvent<HTMLTextAreaElement>) => void
}
