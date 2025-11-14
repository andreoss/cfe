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
  const username = `e2e_profile_${suffix}`
  const email = `${username}@example.com`
  const password = 'correcthorse'
  const bioText = 'Hello from the e2e profile spec.'
  try {
    await driver.get(`${baseUrl}/register`)
    await driver.wait(until.elementLocated(By.name('username')), 5000)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('email')).sendKeys(email)
    await driver.findElement(By.name('password')).sendKeys(password)
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

    await driver.findElement(By.linkText(username)).click()
    await driver.wait(until.urlIs(`${baseUrl}/u/${username}`), 5000)
    let bodyText = await driver.findElement(By.css('body')).getText()
    assert(bodyText.includes('No bio yet.'), 'new profile should show no bio yet')

    await driver.findElement(By.xpath("//button[text()='Edit bio']")).click()
    const textarea = await driver.wait(until.elementLocated(By.name('bio')), 5000)
    await textarea.sendKeys(bioText)
    await driver.findElement(By.xpath("//button[text()='Save']")).click()
    let main = await driver.findElement(By.css('main'))
    await driver.wait(until.elementTextContains(main, bioText), 5000)

    await driver.navigate().refresh()
    main = await driver.findElement(By.css('main'))
    await driver.wait(until.elementTextContains(main, bioText), 5000)
    bodyText = await main.getText()
    assert(bodyText.includes(bioText), 'bio did not persist across reload')

    await driver.get(`${baseUrl}/u/${username}`)
    main = await driver.findElement(By.css('main'))
    await driver.wait(until.elementTextContains(main, bioText), 5000)
    await driver.wait(until.elementLocated(By.xpath("//button[text()='Edit bio']")), 5000)

    console.log('e2e: profile view and edit flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
