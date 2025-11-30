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

async function openNewTopic(driver) {
  let last
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      await clickWhenReady(
        driver,
        By.xpath("//button[text()='New topic']"),
        'the new topic control should be present',
      )
      await driver.wait(until.elementLocated(By.name('title')), 2000)
      return
    } catch (err) {
      last = err
      await driver.navigate().refresh()
    }
  }
  throw last
}

async function postTopic(driver, title) {
  await driver.get(`${baseUrl}/s/general`)
  await openNewTopic(driver)
  await driver.findElement(By.name('title')).sendKeys(title)
  await driver.findElement(By.name('body')).sendKeys('Body for the abuse defence spec.')
  await clickWhenReady(driver, By.xpath("//button[text()='Post']"), 'post control')
  await driver.wait(
    async () => {
      const text = await mainText(driver)
      return text.includes(title) || text.includes('slow down') || text.includes('address is blocked')
    },
    20000,
    'posting should either land or be refused',
  )
  return mainText(driver)
}

async function run() {
  const mod = await buildDriver()
  const user = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_ab_m_${suffix}`
  const userName = `e2e_ab_u_${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await signIn(mod, modName)
    await register(user, userName)

    let text = await postTopic(user, `Slow first ${suffix}`)
    assert(
      text.includes(`Slow first ${suffix}`),
      `a first post should be accepted, saw: ${text}`,
    )

    text = await postTopic(user, `Slow second ${suffix}`)
    assert(
      text.includes('slow down'),
      `a low-standing author's second post should be held back, saw: ${text}`,
    )
    assert(
      !text.includes(`Slow second ${suffix}`),
      'the held-back topic must not appear',
    )

    text = await postTopic(mod, `Moderator first ${suffix}`)
    assert(
      text.includes(`Moderator first ${suffix}`),
      `a moderator's first post should be accepted, saw: ${text}`,
    )
    text = await postTopic(mod, `Moderator second ${suffix}`)
    assert(
      text.includes(`Moderator second ${suffix}`),
      `a moderator must not be slowed, saw: ${text}`,
    )

    await mod.get(`${baseUrl}/settings`)
    const addr = await mod.wait(until.elementLocated(By.name('block-addr')), 10000)
    await addr.clear()
    await addr.sendKeys('127.0.0.1')
    const reason = await mod.findElement(By.name('block-reason'))
    await reason.clear()
    await reason.sendKeys('abuse spec')
    await clickWhenReady(mod, By.xpath("//button[text()='Block address']"), 'block control')
    await mod.wait(
      async () => (await mainText(mod)).includes('127.0.0.1'),
      10000,
      'the blocked address should be listed',
    )

    text = await postTopic(mod, `Blocked moderator ${suffix}`)
    assert(
      text.includes('address is blocked'),
      `an address block should stop even a moderator, saw: ${text}`,
    )

    await mod.get(`${baseUrl}/settings`)
    await clickWhenReady(mod, By.xpath("//button[text()='Lift']"), 'lift control')
    await mod.wait(
      async () => !(await mainText(mod)).includes('abuse spec'),
      10000,
      'lifting should remove the block from the list',
    )

    text = await postTopic(mod, `Unblocked moderator ${suffix}`)
    assert(
      text.includes(`Unblocked moderator ${suffix}`),
      `lifting the block should allow posting again, saw: ${text}`,
    )

    console.log('e2e: abuse slow mode and address block flow passed')
  } finally {
    await mod.quit()
    await user.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
