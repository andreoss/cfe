import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu')
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) builder.setChromeService(new chrome.ServiceBuilder(chromedriverPath))
  return builder.build()
}

async function mainText(driver) {
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      return (await driver.findElement(By.css('main')).getText()).replace(/\s+/g, ' ')
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function settled(driver, wanted) {
  let saw = ''
  await driver.wait(
    async () => {
      saw = await mainText(driver)
      return saw.includes(wanted)
    },
    15000,
    () => `the page should say "${wanted}", said: ${saw}`,
  )
  return saw
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 15000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 15000)
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  try {
    await driver.get(`${baseUrl}/nothing-answers-to-this-${suffix}`)
    const missing = await settled(driver, 'There is nothing here')
    assert(
      missing.includes(`nothing-answers-to-this-${suffix}`),
      `it should repeat the address that was asked for, saw: ${missing}`,
    )
    await driver.wait(
      until.elementLocated(By.linkText('go back to the sections')),
      15000,
      'a dead end must offer a way out',
    )
    await driver.findElement(By.linkText('go back to the sections')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 15000, 'the way out should lead home')

    await driver.get(`${baseUrl}/t/not-a-real-topic-${suffix}`)
    await settled(driver, 'There is nothing here')
    await driver.wait(
      until.elementLocated(By.linkText('go back to the sections')),
      15000,
      'a subject that is not there must offer a way out too',
    )

    for (const [path, wanted] of [
      ['/notifications', 'Sign in to see your notifications.'],
      ['/bookmarks', 'Sign in to see what you have saved.'],
      ['/watched', 'Sign in to see what you are watching.'],
    ]) {
      await driver.get(`${baseUrl}${path}`)
      const text = await settled(driver, wanted)
      assert(
        !/missing session/i.test(text),
        `${path} should not show the wire's words to a person, saw: ${text}`,
      )
    }

    const seen = new Set()
    for (const [path, wanted] of [
      ['/', 'Sections'],
      ['/search', 'Search'],
      ['/archive', 'Archive'],
      ['/settings', 'Settings'],
    ]) {
      await driver.get(`${baseUrl}${path}`)
      await driver.wait(
        async () => (await driver.getTitle()).startsWith(wanted),
        15000,
        `the tab for ${path} should begin with "${wanted}", was "${await driver.getTitle()}"`,
      )
      seen.add(await driver.getTitle())
    }
    assert(seen.size === 4, `each place should name itself in the tab, saw: ${[...seen].join(' / ')}`)

    await driver.get(`${baseUrl}/nothing-here-either-${suffix}`)
    await driver.wait(
      async () => (await driver.getTitle()).startsWith('Nothing here'),
      15000,
      'even a dead end should name itself in the tab',
    )

    await register(driver, `e2e_way_${suffix}`)
    await driver.get(`${baseUrl}/notifications`)
    const signedIn = await settled(driver, 'Notifications')
    assert(
      !signedIn.includes('Sign in to see your notifications.'),
      `somebody signed in should not be asked to sign in, saw: ${signedIn}`,
    )

    await driver.get(`${baseUrl}/settings`)
    const settings = await settled(driver, 'Display')
    assert(
      !/missing session/i.test(settings),
      `settings should not show the wire's words, saw: ${settings}`,
    )

    console.log('e2e: wayfinding flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
