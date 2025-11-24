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

async function attemptSignIn(driver, username, password) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  const name = await driver.findElement(By.name('username'))
  await name.clear()
  await name.sendKeys(username)
  const pass = await driver.findElement(By.name('password'))
  await pass.clear()
  await pass.sendKeys(password)
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(
    async () => {
      if ((await driver.getCurrentUrl()) === `${baseUrl}/`) return true
      return (await driver.findElements(By.css('[role="alert"]'))).length > 0
    },
    10000,
    'sign-in should either succeed or report an error',
  )
  if ((await driver.getCurrentUrl()) === `${baseUrl}/`) return ''
  return mainText(driver)
}

async function run() {
  const here = await buildDriver()
  const elsewhere = await buildDriver()
  const suffix = Date.now().toString(36)
  const username = `e2e_ss_${suffix}`
  try {
    await register(here, username)
    const landed = await attemptSignIn(elsewhere, username, 'correcthorse')
    assert(landed === '', `the second session should sign in, saw: ${landed}`)

    await here.get(`${baseUrl}/settings`)
    await clickWhenReady(
      here,
      By.xpath("//button[normalize-space(.)='End all sessions']"),
      'the end all sessions control should be present',
    )
    await clickWhenReady(
      here,
      By.xpath("//button[normalize-space(.)='Confirm end all sessions']"),
      'the confirmation should appear',
    )
    await here.wait(until.urlIs(`${baseUrl}/`), 10000, 'ending sessions should land home')

    await here.get(`${baseUrl}/settings`)
    await here.wait(
      async () => (await mainText(here)).includes('Sign in to manage your account.'),
      10000,
      'the account that ended its sessions should be signed out here',
    )

    await elsewhere.get(`${baseUrl}/settings`)
    await elsewhere.wait(
      async () => (await mainText(elsewhere)).includes('Sign in to manage your account.'),
      10000,
      'the other session should be ended too',
    )

    let text = ''
    for (let attempt = 0; attempt < 3; attempt += 1) {
      text = await attemptSignIn(here, username, 'notthepassword')
      assert(
        text.toLowerCase().includes('invalid credentials'),
        `a wrong password should be reported, saw: ${text}`,
      )
    }

    text = await attemptSignIn(here, username, 'notthepassword')
    assert(
      text.includes('Too many attempts. Try again later.'),
      `further attempts should be throttled, saw: ${text}`,
    )

    text = await attemptSignIn(here, username, 'correcthorse')
    assert(
      text.includes('Too many attempts. Try again later.'),
      `the throttle should hold even for the right password, saw: ${text}`,
    )

    console.log('e2e: session security flow passed')
  } finally {
    await here.quit()
    await elsewhere.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
