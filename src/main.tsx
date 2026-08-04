import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from './app/App'
import './styles.css'
import './runtime.css'
import './entity.css'
import './provenance.css'
import './log-window.css'
import './mining.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
