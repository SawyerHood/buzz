import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/')({ component: LandingPage })

/* ─── data ───────────────────────────────────────────────────────── */

const features = [
  {
    emoji: '🎤',
    title: 'Instant Transcription',
    desc: 'OpenAI-powered speech-to-text with realtime streaming. Your words appear as fast as you speak them.',
  },
  {
    emoji: '⌨️',
    title: 'Global Hotkeys',
    desc: 'Hold-to-talk, toggle, or double-tap — pick your style. Supports Fn key, Right Alt, and custom combos.',
  },
  {
    emoji: '📋',
    title: 'Auto-Insert',
    desc: 'Text appears right where your cursor is. Works in any app — editors, browsers, Slack, everywhere.',
  },
  {
    emoji: '🔐',
    title: 'Flexible Auth',
    desc: 'Log in with your ChatGPT subscription via OAuth — no API key needed. Or bring your own key.',
  },
  {
    emoji: '📊',
    title: 'Usage Stats',
    desc: 'Track your words dictated, words-per-minute, and daily streaks with a built-in dashboard.',
  },
  {
    emoji: '🪶',
    title: 'Lightweight & Native',
    desc: 'Built with Tauri v2 and Rust — not Electron. Tiny footprint, instant launch, zero bloat.',
  },
]

const steps = [
  {
    num: '1',
    icon: '⌨️',
    title: 'Press your hotkey',
    desc: 'A floating overlay appears so you know Buzz is listening.',
  },
  {
    num: '2',
    icon: '🗣️',
    title: 'Speak naturally',
    desc: 'Talk at your normal pace. Buzz streams your speech to OpenAI in realtime.',
  },
  {
    num: '3',
    icon: '✨',
    title: 'Text appears',
    desc: 'Your transcript is inserted right at the cursor position — instantly.',
  },
]

/* ─── component ──────────────────────────────────────────────────── */

function LandingPage() {
  return (
    <div className="min-h-screen">
      <Nav />
      <Hero />
      <Features />
      <HowItWorks />
      <TechStack />
      <Footer />
    </div>
  )
}

/* ─── nav ────────────────────────────────────────────────────────── */

function Nav() {
  return (
    <nav className="sticky top-0 z-50 backdrop-blur-md bg-amber-50/80 border-b border-amber-200/60">
      <div className="max-w-6xl mx-auto px-6 h-16 flex items-center justify-between">
        <a href="#" className="flex items-center gap-2 text-xl font-bold text-gray-900 no-underline">
          <span className="text-2xl" role="img" aria-label="bee">🐝</span>
          Buzz
        </a>
        <div className="flex items-center gap-6">
          <a href="#features" className="hidden sm:inline text-sm text-gray-600 hover:text-gray-900 no-underline transition-colors">Features</a>
          <a href="#how-it-works" className="hidden sm:inline text-sm text-gray-600 hover:text-gray-900 no-underline transition-colors">How it works</a>
          <a
            href="https://github.com/SawyerHood/voice"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5 text-sm font-medium text-gray-700 hover:text-gray-900 no-underline transition-colors"
          >
            <GitHubIcon />
            GitHub
          </a>
        </div>
      </div>
    </nav>
  )
}

/* ─── hero ───────────────────────────────────────────────────────── */

function Hero() {
  return (
    <header className="relative overflow-hidden">
      {/* Honeycomb background pattern */}
      <div className="absolute inset-0 opacity-[0.04]" style={{ backgroundImage: honeycombSvg, backgroundSize: '60px 52px' }} />

      <div className="relative max-w-6xl mx-auto px-6 pt-12 pb-20 md:pt-16 md:pb-28">
        {/* Header image */}
        <div className="flex justify-center mb-10">
          <img
            src="/header.png"
            alt="Buzz — a friendly bee mascot with headphones"
            width={1376}
            height={768}
            className="w-full max-w-3xl rounded-2xl shadow-2xl shadow-amber-900/20"
          />
        </div>

        <div className="text-center max-w-3xl mx-auto">
          <h1 className="text-4xl sm:text-5xl md:text-6xl font-extrabold tracking-tight text-gray-900 mb-6 leading-[1.1]">
            Voice-to-text with a{' '}
            <span className="bg-gradient-to-r from-amber-500 to-amber-600 bg-clip-text text-transparent">
              quick buzz
            </span>
          </h1>

          <p className="text-lg sm:text-xl text-gray-600 mb-10 max-w-2xl mx-auto leading-relaxed">
            A tiny macOS menubar app that turns your speech into text — right where your cursor is.
            Press a hotkey, talk, and your words appear instantly.
          </p>

          <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
            <a
              href="#"
              className="inline-flex items-center gap-2 px-7 py-3.5 rounded-xl bg-gray-900 text-white font-semibold text-base shadow-lg shadow-gray-900/25 hover:bg-gray-800 hover:shadow-xl hover:shadow-gray-900/30 transition-all no-underline active:scale-[0.98]"
            >
              <AppleIcon />
              Download for macOS
            </a>
            <a
              href="https://github.com/SawyerHood/voice"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 px-7 py-3.5 rounded-xl bg-white text-gray-700 font-semibold text-base border border-gray-200 shadow-sm hover:border-gray-300 hover:shadow-md transition-all no-underline active:scale-[0.98]"
            >
              <GitHubIcon />
              View on GitHub
            </a>
          </div>
        </div>
      </div>
    </header>
  )
}

/* ─── features ───────────────────────────────────────────────────── */

function Features() {
  return (
    <section id="features" className="relative py-20 md:py-28 bg-white">
      <div className="max-w-6xl mx-auto px-6">
        <div className="text-center mb-14">
          <h2 className="text-3xl sm:text-4xl font-bold text-gray-900 mb-4">
            Everything you need to dictate
          </h2>
          <p className="text-lg text-gray-500 max-w-2xl mx-auto">
            Buzz is packed with thoughtful features so you can focus on speaking — not fiddling with settings.
          </p>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
          {features.map((f) => (
            <article
              key={f.title}
              className="group rounded-2xl border border-gray-100 bg-gray-50/50 p-6 hover:border-amber-200 hover:bg-amber-50/40 transition-all duration-200"
            >
              <div className="text-3xl mb-4" role="img" aria-label={f.title}>{f.emoji}</div>
              <h3 className="text-lg font-semibold text-gray-900 mb-2">{f.title}</h3>
              <p className="text-gray-500 leading-relaxed text-[0.95rem]">{f.desc}</p>
            </article>
          ))}
        </div>
      </div>
    </section>
  )
}

/* ─── how it works ───────────────────────────────────────────────── */

function HowItWorks() {
  return (
    <section id="how-it-works" className="relative py-20 md:py-28 bg-amber-50">
      {/* Subtle honeycomb bg */}
      <div className="absolute inset-0 opacity-[0.03]" style={{ backgroundImage: honeycombSvg, backgroundSize: '60px 52px' }} />

      <div className="relative max-w-5xl mx-auto px-6">
        <div className="text-center mb-14">
          <h2 className="text-3xl sm:text-4xl font-bold text-gray-900 mb-4">
            How it works
          </h2>
          <p className="text-lg text-gray-500 max-w-xl mx-auto">
            Three steps. No setup headaches.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8 md:gap-12">
          {steps.map((s, i) => (
            <div key={s.num} className="text-center">
              {/* Step number */}
              <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-amber-100 text-3xl mb-5">
                {s.icon}
              </div>
              <h3 className="text-xl font-semibold text-gray-900 mb-2">
                <span className="text-amber-500 font-bold mr-1">{s.num}.</span>
                {s.title}
              </h3>
              <p className="text-gray-500 leading-relaxed">{s.desc}</p>
              {/* Connector arrow (hidden on last item and mobile) */}
              {i < steps.length - 1 && (
                <div className="hidden md:block absolute" aria-hidden="true" />
              )}
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}

/* ─── tech stack ─────────────────────────────────────────────────── */

function TechStack() {
  const tech = [
    { name: 'Tauri v2', desc: 'Native desktop shell' },
    { name: 'React', desc: 'UI framework' },
    { name: 'Rust', desc: 'Backend & system access' },
    { name: 'TypeScript', desc: 'Type-safe frontend' },
    { name: 'OpenAI', desc: 'Transcription engine' },
  ]

  return (
    <section className="py-16 md:py-20 bg-white border-t border-gray-100">
      <div className="max-w-4xl mx-auto px-6 text-center">
        <h2 className="text-2xl font-bold text-gray-900 mb-8">Built with</h2>
        <div className="flex flex-wrap justify-center gap-4">
          {tech.map((t) => (
            <div
              key={t.name}
              className="flex items-center gap-2 px-4 py-2 rounded-full bg-gray-50 border border-gray-100 text-sm"
            >
              <span className="font-semibold text-gray-800">{t.name}</span>
              <span className="text-gray-400">·</span>
              <span className="text-gray-500">{t.desc}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}

/* ─── footer ─────────────────────────────────────────────────────── */

function Footer() {
  return (
    <footer className="py-10 bg-gray-900 text-gray-400">
      <div className="max-w-6xl mx-auto px-6 flex flex-col sm:flex-row items-center justify-between gap-4">
        <div className="flex items-center gap-2 text-sm">
          <span role="img" aria-label="bee">🐝</span>
          <span>
            Built by{' '}
            <a
              href="https://sawyerhood.com"
              target="_blank"
              rel="noopener noreferrer"
              className="text-amber-400 hover:text-amber-300 no-underline transition-colors"
            >
              Sawyer Hood
            </a>
          </span>
        </div>
        <a
          href="https://github.com/SawyerHood/voice"
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1.5 text-sm text-gray-400 hover:text-white no-underline transition-colors"
        >
          <GitHubIcon />
          GitHub
        </a>
      </div>
    </footer>
  )
}

/* ─── icons ──────────────────────────────────────────────────────── */

function GitHubIcon() {
  return (
    <svg className="w-4 h-4" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  )
}

function AppleIcon() {
  return (
    <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.8-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z" />
    </svg>
  )
}

/* ─── honeycomb SVG pattern (inline data URI) ────────────────────── */

const honeycombSvg = `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='56' height='100' viewBox='0 0 56 100'%3E%3Cpath d='M28 66L0 50L0 16L28 0L56 16L56 50L28 66ZM28 100L0 84L0 50L28 34L56 50L56 84L28 100Z' fill='none' stroke='%23b45309' stroke-width='1'/%3E%3C/svg%3E")`
