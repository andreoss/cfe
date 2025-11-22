import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot } from './support.mjs'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu')
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) {
    const service = new chrome.ServiceBuilder(chromedriverPath)
    builder.setChromeService(service)
  }
  return builder.build()
}

async function mainText(driver) {
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      return await driver.findElement(By.css('main')).getText()
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function clickWhenReady(driver, locator, message) {
  await driver.wait(until.elementLocated(locator), 10000, message)
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      await driver.findElement(locator).click()
      return
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function signIn(driver, username) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function postTopic(driver, title) {
  await driver.get(`${baseUrl}/s/general`)
  await clickWhenReady(driver, By.xpath("//button[text()='New topic']"), 'new topic control')
  await driver.findElement(By.name('title')).sendKeys(title)
  await driver.findElement(By.name('body')).sendKeys('Body for the rate limit spec.')
  await clickWhenReady(driver, By.xpath("//button[text()='Post']"), 'post control')
  let outcome = ''
  await driver.wait(
    async () => {
      const text = await mainText(driver)
      if (text.includes(title) || text.includes('slow down')) {
        outcome = text
        return true
      }
      return false
    },
    10000,
    'posting should either land or be refused',
  )
  return outcome
}

async function postUntilAccepted(driver, prefix) {
  for (let attempt = 0; attempt < 30; attempt += 1) {
    const text = await postTopic(driver, `${prefix} ${attempt}`)
    if (text.includes(`${prefix} ${attempt}`)) return
    await new Promise((resolve) => setTimeout(resolve, 1000))
  }
  throw new Error(`${prefix} never got through the rate window`)
}

async function run() {
  const first = await buildDriver()
  const second = await buildDriver()
  const mod = await buildDriver()
  const suffix = Date.now().toString(36)
  const firstName = `e2e_rt_a_${suffix}`
  const secondName = `e2e_rt_b_${suffix}`
  const modName = `e2e_rt_m_${suffix}`
  try {
    await register(first, firstName)
    await register(second, secondName)
    await register(mod, modName)
    await promoteViaRoot(modName)
    await signIn(mod, modName)

    await postUntilAccepted(first, `Rate warmup ${suffix}`)

    let text = await postTopic(second, `Rate two ${suffix}`)
    assert(text.includes(`Rate two ${suffix}`), `the second post should land, saw: ${text}`)

    text = await postTopic(second, `Rate three ${suffix}`)
    assert(
      text.includes('slow down'),
      `a third post from the same address should be refused, saw: ${text}`,
    )
    assert(
      !text.includes(`Rate three ${suffix}`),
      'the refused topic must not appear',
    )

    text = await postTopic(first, `Rate four ${suffix}`)
    assert(
      text.includes('slow down'),
      `the limit counts the address, not the account, so another user is refused too, saw: ${text}`,
    )

    text = await postTopic(mod, `Rate moderator ${suffix}`)
    assert(
      text.includes(`Rate moderator ${suffix}`),
      `a moderator must not be rate limited, saw: ${text}`,
    )

    console.log('e2e: abuse rate limit flow passed')
  } finally {
    await first.quit()
    await second.quit()
    await mod.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
