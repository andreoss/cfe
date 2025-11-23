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

async function runSearch(driver, term) {
  await driver.get(`${baseUrl}/search`)
  const box = await driver.wait(until.elementLocated(By.name('query')), 5000)
  await box.clear()
  await box.sendKeys(term)
  await driver.findElement(By.xpath("//button[normalize-space(text())='Search']")).click()
  await driver.wait(until.urlContains('q='), 5000)
  await driver.sleep(500)
  return driver.findElement(By.css('body')).getText()
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const username = `e2e_search_${suffix}`
  const term = `zylophone${suffix}`
  const topicTitle = `Findable ${term} subject`
  const commentBody = `A comment mentioning ${term} inside it`
  try {
    await driver.get(`${baseUrl}/register`)
    await driver.wait(until.elementLocated(By.name('username')), 5000)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
    await driver.findElement(By.name('password')).sendKeys('correcthorse')
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

    let text = await runSearch(driver, term)
    assert(text.includes('Nothing found.'), 'an unused term should find nothing')

    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(until.elementLocated(By.xpath("//button[text()='New topic']")), 5000)
    await driver.findElement(By.xpath("//button[text()='New topic']")).click()
    await driver.wait(until.elementLocated(By.name('title')), 10000)
    await driver.findElement(By.name('title')).sendKeys(topicTitle)
    await driver.findElement(By.name('body')).sendKeys('Body text for the search e2e spec.')
    await driver.findElement(By.xpath("//button[text()='Post']")).click()
    await publishTopic(topicTitle)
    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(until.elementLocated(By.linkText(topicTitle)), 5000)

    text = await runSearch(driver, term)
    assert(text.includes(topicTitle), 'the new topic should be found by its title term')
    assert(!text.includes('Nothing found.'), 'a matching search should not say nothing found')

    await driver.findElement(By.linkText(topicTitle)).click()
    await driver.wait(until.elementLocated(By.name('comment-body')), 5000)
    await driver.findElement(By.name('comment-body')).sendKeys(commentBody)
    await driver.findElement(By.xpath("//button[text()='Post comment']")).click()
    await driver.wait(until.elementLocated(By.xpath(`//p[contains(., '${commentBody}')]`)), 5000)

    text = await runSearch(driver, term)
    assert(text.includes(topicTitle), 'the topic should still be found')
    assert(text.includes(`Comment by ${username}`), 'the matching comment should be found too')

    await driver.findElement(By.linkText(topicTitle)).click()
    await driver.wait(until.elementLocated(By.name('comment-body')), 5000)
    const heading = await driver.findElement(By.css('h1')).getText()
    assert(heading.includes(topicTitle), 'a result should link back to its topic')

    console.log('e2e: search flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
