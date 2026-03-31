import { useState } from 'react'
import type { FormEvent } from 'react'
import { Icon } from '@iconify/react'
import { Button, Card, CardBody, Form, Input } from '@heroui/react'
import { isAxiosError } from 'axios'
import { useTranslation } from 'react-i18next'
import { motion } from 'framer-motion'
import { LanguageToggle } from '@/components/LanguageToggle'
import { notify } from '@/lib/notification'
import { ThemeToggleButton } from '@/components/ui/theme-toggle-button'
import SoftAurora from '@/components/ui/soft-aurora'

interface LoginProps {
  onLogin: (username: string, password: string) => Promise<void>
}

export default function Login({ onLogin }: LoginProps) {
  const { t } = useTranslation()
  const [username, setUsername] = useState('admin')
  const [password, setPassword] = useState('')
  const [loading, setLoading] = useState(false)
  const [showPassword, setShowPassword] = useState(false)

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    setLoading(true)
    try {
      await onLogin(username.trim(), password)
    } catch (err: unknown) {
      if (!isAxiosError(err) || err.response?.status !== 401) {
        notify({
          variant: 'error',
          title: t('login.messages.failed'),
        })
      }
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="relative flex min-h-screen items-center justify-center overflow-hidden bg-background px-4 py-10">
      {/* SoftAurora 背景 — 保持品牌感但比原版更克制 */}
      <div
        className="pointer-events-none absolute inset-0 opacity-35 dark:opacity-25"
        aria-hidden="true"
      >
        <SoftAurora
          color1="#0d9488"
          color2="#2dd4bf"
          speed={0.35}
          scale={1.1}
          brightness={0.75}
          bandHeight={0.5}
          bandSpread={1.0}
          noiseFrequency={1.8}
          noiseAmplitude={0.6}
          layerOffset={0.35}
          colorSpeed={0.5}
          enableMouseInteraction={false}
        />
      </div>

      <div className="absolute right-4 top-4 z-20 flex items-center gap-1">
        <ThemeToggleButton />
        <LanguageToggle />
      </div>

      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
        className="relative z-10 w-full max-w-sm"
      >
        <Card className="border-small border-default-200 bg-content1/80 shadow-medium backdrop-blur-lg">
          <CardBody className="px-6 py-8">
            {/* 品牌标识 */}
            <div className="mb-8 flex items-center gap-3">
              <img src="/favicon.svg" alt="Codex-Pool" className="h-9 w-9 rounded-xl" />
              <div>
                <p className="text-xs font-semibold uppercase tracking-widest text-default-400">
                  Codex
                </p>
                <p className="text-sm font-semibold leading-none text-foreground">Pool</p>
              </div>
            </div>

            {/* 标题区 */}
            <div className="mb-6 space-y-1.5">
              <h1 className="text-2xl font-semibold tracking-[-0.02em] text-foreground">
                {t('login.title')}
              </h1>
              <p className="text-sm text-default-500">
                {t('login.subtitle')}
              </p>
            </div>

            <Form
              className="flex flex-col gap-3"
              validationBehavior="native"
              onSubmit={submit}
            >
              <Input
                isRequired
                autoFocus
                autoComplete="username"
                label={t('login.username')}
                labelPlacement="outside"
                name="username"
                placeholder={t('login.usernamePlaceholder')}
                size="md"
                value={username}
                onValueChange={setUsername}
              />

              <Input
                isRequired
                autoComplete="current-password"
                label={t('login.password')}
                labelPlacement="outside"
                name="password"
                placeholder={t('login.passwordPlaceholder')}
                size="md"
                type={showPassword ? 'text' : 'password'}
                value={password}
                onValueChange={setPassword}
                endContent={(
                  <button
                    type="button"
                    className="text-default-400 transition-colors hover:text-foreground focus:outline-none"
                    aria-label={
                      showPassword
                        ? t('login.hidePassword')
                        : t('login.showPassword')
                    }
                    onClick={() => setShowPassword((c) => !c)}
                  >
                    <Icon
                      icon={showPassword ? 'solar:eye-bold' : 'solar:eye-closed-linear'}
                      className="text-lg"
                    />
                  </button>
                )}
              />

              <Button
                className="mt-2 w-full font-medium"
                color="primary"
                isDisabled={!username.trim() || !password}
                isLoading={loading}
                size="md"
                type="submit"
              >
                {t('login.submit')}
              </Button>
            </Form>
          </CardBody>
        </Card>
      </motion.div>
    </div>
  )
}
