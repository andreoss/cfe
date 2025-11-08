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

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const username = `e2e_${suffix}`
  const email = `${username}@example.com`
  const password = 'correcthorse'
  try {
    await driver.get(`${baseUrl}/register`)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('email')).sendKeys(email)
    await driver.findElement(By.name('password')).sendKeys(password)
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)
    let bodyText = await driver.findElement(By.css('body')).getText()
    assert(bodyText.includes(username), 'registered user is not shown on home page')

    await driver.get(`${baseUrl}/sign-in`)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('password')).sendKeys(password)
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)
    bodyText = await driver.findElement(By.css('body')).getText()
    assert(bodyText.includes(username), 'signed-in user is not shown on home page')

    await driver.navigate().refresh()
    await driver.wait(until.elementLocated(By.xpath("//button[text()='Sign out']")), 5000)
    bodyText = await driver.findElement(By.css('body')).getText()
    assert(bodyText.includes(username), 'session did not persist across reload')

    await driver.findElement(By.xpath("//button[text()='Sign out']")).click()
    await driver.wait(until.elementLocated(By.linkText('Sign in')), 5000)

    await driver.navigate().refresh()
    await driver.wait(until.elementLocated(By.linkText('Sign in')), 5000)
    bodyText = await driver.findElement(By.css('body')).getText()
    assert(!bodyText.includes(username), 'session persisted after sign-out')

    await driver.get(`${baseUrl}/sign-in`)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('password')).sendKeys('wrong-password')
    await driver.findElement(By.css('button[type="submit"]')).click()
    const alert = await driver.wait(until.elementLocated(By.css('[role="alert"]')), 5000)
    const alertText = await alert.getText()
    assert(alertText.length > 0, 'wrong password did not show an error')

    console.log('e2e: register, session persistence, sign-out, sign-in flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
