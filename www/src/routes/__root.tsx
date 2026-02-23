import { HeadContent, Scripts, createRootRoute } from '@tanstack/react-router'
import appCss from '../styles.css?url'

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: 'utf-8' },
      { name: 'viewport', content: 'width=device-width, initial-scale=1' },
      { title: 'Buzz — Voice-to-text for macOS' },
      { name: 'description', content: 'Lightning-fast voice-to-text that types where your cursor is. Press your hotkey, speak, and Buzz inserts the transcript. Native macOS app powered by OpenAI.' },
      { property: 'og:title', content: 'Buzz 🐝 — Voice-to-text for macOS' },
      { property: 'og:description', content: 'Lightning-fast voice-to-text that types where your cursor is.' },
      { property: 'og:image', content: '/header.png' },
      { property: 'og:type', content: 'website' },
      { name: 'theme-color', content: '#d97706' },
    ],
    links: [
      { rel: 'stylesheet', href: appCss },
      { rel: 'icon', href: '/favicon.ico' },
      { rel: 'preconnect', href: 'https://fonts.googleapis.com' },
      { rel: 'preconnect', href: 'https://fonts.gstatic.com', crossOrigin: 'anonymous' },
      { rel: 'stylesheet', href: 'https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap' },
    ],
  }),
  shellComponent: RootDocument,
})

function RootDocument({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <head>
        <HeadContent />
      </head>
      <body className="bg-[#fafafa] text-neutral-900 antialiased">
        {children}
        <Scripts />
      </body>
    </html>
  )
}
