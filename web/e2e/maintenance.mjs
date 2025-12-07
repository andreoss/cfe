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
  await driver.wait(
    async () => {
      if ((await driver.getCurrentUrl()) === `${baseUrl}/`) return true
      return (await driver.findElements(By.css('[role="alert"]'))).length > 0
    },
    10000,
    'sign-in should either succeed or report an error',
  )
  return driver.getCurrentUrl()
}

async function runMaintenance(driver) {
  await driver.get(`${baseUrl}/operator`)
  await clickWhenReady(
    driver,
    By.xpath("//button[text()='Run maintenance']"),
    'a moderator should be offered the maintenance control',
  )
  let report = null
  await driver.wait(
    async () => {
      const match = (await mainText(driver)).match(/Blocked (\d+), dropped (\d+)\./)
      if (match === null) return false
      report = { blocked: Number(match[1]), dropped: Number(match[2]) }
      return true
    },
    10000,
    'running maintenance should report what it did',
  )
  return report
}

async function run() {
  const mod = await buildDriver()
  const doomed = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_mt_m_${suffix}`
  const doomedName = `e2e_mt_d_${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await signIn(mod, modName)
    await register(doomed, doomedName)

    await doomed.get(`${baseUrl}/operator`)
    await doomed.wait(
      async () =>
        (await doomed.findElement(By.css('main')).getText()).includes(
          'Only a moderator can run the board.',
        ),
      10000,
      'a plain user must be turned away from the operator surface',
    )
    const noControl = await doomed.findElements(
      By.xpath("//button[text()='Run maintenance']"),
    )
    assert(noControl.length === 0, 'a plain user must not be offered the maintenance control')

    let refusedAt = ''
    for (let attempt = 0; attempt < 30; attempt += 1) {
      await runMaintenance(mod)
      const landed = await signIn(doomed, doomedName)
      if (landed !== `${baseUrl}/`) {
        refusedAt = await mainText(doomed)
        break
      }
      await new Promise((resolve) => setTimeout(resolve, 1000))
    }
    assert(
      refusedAt !== '',
      'the run should eventually drop the unconfirmed account so it cannot sign in',
    )
    assert(
      refusedAt.toLowerCase().includes('invalid credentials'),
      `a dropped account should be refused, saw: ${refusedAt}`,
    )

    await mod.get(`${baseUrl}/operator`)
    await mod.wait(
      until.elementLocated(By.xpath("//button[text()='Run maintenance']")),
      10000,
      'the moderator should still be signed in after the run',
    )

    console.log('e2e: maintenance flow passed')
  } finally {
    await mod.quit()
    await doomed.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
