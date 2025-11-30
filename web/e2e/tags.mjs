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
  const username = `e2e_tags_${suffix}`
  const email = `${username}@example.com`
  const password = 'correcthorse'
  const tag = `e2etag${suffix}`
  const topicTitle = `Tagged topic ${suffix}`
  try {
    await driver.get(`${baseUrl}/register`)
    await driver.wait(until.elementLocated(By.name('username')), 5000)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('email')).sendKeys(email)
    await driver.findElement(By.name('password')).sendKeys(password)
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

    await driver.wait(until.elementLocated(By.linkText('General')), 10000)


    await driver.findElement(By.linkText('General')).click()
    await driver.wait(until.urlIs(`${baseUrl}/s/general`), 5000)
    await driver.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'the new topic control should appear once posting is known to be allowed',
    )
    await driver.findElement(By.xpath("//button[text()='New topic']")).click()
    await driver.wait(until.elementLocated(By.name('title')), 10000)
    await driver.findElement(By.name('title')).sendKeys(topicTitle)
    await driver.findElement(By.name('body')).sendKeys('Body for the tags e2e spec.')
    await driver.findElement(By.name('tags')).sendKeys(`${tag}, ${tag}, other`)
    await driver.findElement(By.xpath("//button[text()='Post']")).click()
    await publishTopic(topicTitle)
    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(until.elementLocated(By.linkText(topicTitle)), 5000)

    let bodyText = await driver.findElement(By.css('body')).getText()
    assert(bodyText.includes(tag), 'section listing should show the tag chip')

    await driver.findElement(By.linkText(tag)).click()
    await driver.wait(until.urlIs(`${baseUrl}/tag/${tag}`), 5000)
    await driver.wait(until.elementLocated(By.linkText(topicTitle)), 5000)
    bodyText = await driver.findElement(By.css('body')).getText()
    assert(bodyText.includes(topicTitle), 'tag page should list the tagged topic')

    await driver.get(`${baseUrl}/tag/doesnotexist${suffix}`)
    await driver.wait(until.elementTextContains(await driver.findElement(By.css('main')), 'No topics'), 5000)

    console.log('e2e: tags flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
