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
  const username = `e2e_markdown_${suffix}`
  const email = `${username}@example.com`
  const password = 'correcthorse'
  const topicTitle = `Markdown topic ${suffix}`
  const xssMarker = `xss-e2e-${suffix}`
  try {
    await driver.get(`${baseUrl}/register`)
    await driver.executeScript(`window.__alertFired = false;
      window.alert = function() { window.__alertFired = true; };`)
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
    await driver
      .findElement(By.name('body'))
      .sendKeys(`Check **bold** and a [link](https://example.com) here.`)
    await driver.findElement(By.xpath("//button[text()='Post']")).click()
    await publishTopic(topicTitle)
    await driver.findElement(By.linkText('Home')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)
    await driver.wait(until.elementLocated(By.linkText('General')), 10000)

    await driver.findElement(By.linkText('General')).click()
    await driver.wait(until.urlIs(`${baseUrl}/s/general`), 5000)
    await driver.wait(until.elementLocated(By.linkText(topicTitle)), 5000)
    await driver.findElement(By.linkText(topicTitle)).click()
    await driver.wait(until.elementLocated(By.css('.body')), 5000)

    const strongEl = await driver.findElement(By.css('.body strong'))
    assert((await strongEl.getText()) === 'bold', 'bold markdown should render as <strong>')
    const linkEl = await driver.findElement(By.css('.body a'))
    assert((await linkEl.getAttribute('href')) === 'https://example.com/', 'link should render as <a href>')

    await driver.findElement(By.name('comment-body')).sendKeys(`<script>alert('${xssMarker}')</script>`)
    await driver.findElement(By.xpath("//button[text()='Post comment']")).click()
    await driver.wait(until.elementLocated(By.css('ul li .body')), 5000)

    const alertFired = await driver.executeScript('return window.__alertFired')
    assert(alertFired === false, 'XSS payload must not execute as script')

    const scripts = await driver.findElements(By.css('script'))
    for (const s of scripts) {
      const text = await s.getAttribute('innerHTML')
      assert(!text.includes(xssMarker), 'XSS marker must not appear inside any <script> tag')
    }
    const pageSource = await driver.getPageSource()
    assert(
      !pageSource.includes(`<script>alert('${xssMarker}')`),
      'raw script tag must not appear in the rendered page source',
    )

    console.log('e2e: markdown rendering and XSS sanitization passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
