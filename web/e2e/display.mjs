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

async function choose(driver, name, value) {
  await driver.wait(until.elementLocated(By.name(name)), 15000, `the ${name} control`)
  const select = await driver.findElement(By.name(name))
  await select.findElement(By.css(`option[value="${value}"]`)).click()
}

async function themeAttribute(driver) {
  return driver.executeScript('return document.documentElement.getAttribute("data-theme")')
}

async function densityAttribute(driver) {
  return driver.executeScript('return document.documentElement.getAttribute("data-density")')
}

async function run() {
  const driver = await buildDriver()
  try {
    await driver.get(`${baseUrl}/settings`)
    await driver.wait(
      async () => (await driver.findElement(By.css('main')).getText()).includes('Display'),
      15000,
      'a reader who is not signed in may still say how the board should look',
    )

    assert(
      (await themeAttribute(driver)) === null,
      'a reader who has said nothing follows their system',
    )

    await choose(driver, 'theme', 'dark')
    await driver.wait(
      async () => (await themeAttribute(driver)) === 'dark',
      15000,
      'choosing dark should take effect at once',
    )

    await choose(driver, 'theme', 'light')
    await driver.wait(
      async () => (await themeAttribute(driver)) === 'light',
      15000,
      'choosing light should take effect at once',
    )

    await choose(driver, 'density', 'compact')
    await driver.wait(
      async () => (await densityAttribute(driver)) === 'compact',
      15000,
      'choosing compact spacing should take effect at once',
    )

    await driver.get(`${baseUrl}/`)
    await driver.wait(
      async () => (await themeAttribute(driver)) === 'light',
      15000,
      'the choice should hold on another page',
    )
    assert(
      (await densityAttribute(driver)) === 'compact',
      'the spacing should hold on another page too',
    )

    await driver.navigate().refresh()
    await driver.wait(
      async () => (await themeAttribute(driver)) === 'light',
      15000,
      'the choice should survive a reload',
    )

    await driver.get(`${baseUrl}/settings`)
    const kept = await driver.findElement(By.name('theme')).getAttribute('value')
    assert(kept === 'light', `the control should show what was chosen, showed: ${kept}`)

    await choose(driver, 'theme', 'system')
    await driver.wait(
      async () => (await themeAttribute(driver)) === null,
      15000,
      'going back to the system setting should stop overriding it',
    )

    console.log('e2e: display settings flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
