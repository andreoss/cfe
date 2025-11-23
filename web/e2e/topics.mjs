import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { publishTopic } from './support.mjs'

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
  const username = `e2e_topics_${suffix}`
  const email = `${username}@example.com`
  const password = 'correcthorse'
  const topicTitle = `Topic ${suffix}`
  const topicBody = 'Body written by the e2e topics spec.'
  try {
    await driver.get(`${baseUrl}/`)
    await driver.wait(until.elementLocated(By.linkText('General')), 5000)
    let bodyText = await driver.findElement(By.css('body')).getText()
    assert(bodyText.includes('General'), 'sections list should show General')
    assert(bodyText.includes('Help'), 'sections list should show Help')
    assert(bodyText.includes('Feedback'), 'sections list should show Feedback')

    await driver.get(`${baseUrl}/register`)
    await driver.wait(until.elementLocated(By.name('username')), 5000)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('email')).sendKeys(email)
    await driver.findElement(By.name('password')).sendKeys(password)
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

    await driver.findElement(By.linkText('General')).click()
    await driver.wait(until.urlIs(`${baseUrl}/s/general`), 5000)

    await driver.findElement(By.xpath("//button[text()='New topic']")).click()
    await driver.wait(until.elementLocated(By.name('title')), 10000)
    await driver.findElement(By.name('title')).sendKeys(topicTitle)
    await driver.findElement(By.name('body')).sendKeys(topicBody)
    await driver.findElement(By.xpath("//button[text()='Post']")).click()
    await publishTopic(topicTitle)
    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(until.elementLocated(By.linkText(topicTitle)), 5000)

    await driver.findElement(By.linkText(topicTitle)).click()
    await driver.wait(
      async () => (await driver.findElement(By.css('body')).getText()).includes(topicBody),
      10000,
      'the topic page should load the topic it links to',
    )
    bodyText = await driver.findElement(By.css('body')).getText()
    assert(bodyText.includes(topicTitle), 'topic page should show the title')
    assert(bodyText.includes(topicBody), 'topic page should show the body')
    assert(bodyText.includes(username), 'topic page should show the author')

    await driver.get(`${baseUrl}/s/help`)
    await driver.wait(until.elementLocated(By.xpath("//button[text()='New topic']")), 5000)

    console.log('e2e: sections and topics flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
