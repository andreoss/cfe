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
  const username = `e2e_comments_${suffix}`
  const email = `${username}@example.com`
  const password = 'correcthorse'
  const topicTitle = `Comments topic ${suffix}`
  const rootComment = `Root comment ${suffix}`
  const replyComment = `Reply comment ${suffix}`
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
    await driver.findElement(By.xpath("//button[text()='New topic']")).click()
    await driver.wait(until.elementLocated(By.name('title')), 10000)
    await driver.findElement(By.name('title')).sendKeys(topicTitle)
    await driver.findElement(By.name('body')).sendKeys('Body for the comments e2e spec.')
    await driver.findElement(By.xpath("//button[text()='Post']")).click()
    await publishTopic(topicTitle)
    await driver.get(`${baseUrl}/s/general`)
    await driver.wait(until.elementLocated(By.linkText(topicTitle)), 5000)
    await driver.findElement(By.linkText(topicTitle)).click()
    await driver.wait(until.elementLocated(By.name('comment-body')), 5000)

    let bodyText = await driver.findElement(By.css('body')).getText()
    assert(bodyText.includes('No comments yet.'), 'new topic should have no comments yet')

    await driver.findElement(By.name('comment-body')).sendKeys(rootComment)
    await driver.findElement(By.xpath("//button[text()='Post comment']")).click()
    await driver.wait(until.elementLocated(By.xpath(`//p[contains(., '${rootComment}')]`)), 5000)

    await driver.findElement(By.xpath("//button[normalize-space(text())='Reply']")).click()
    const replyBox = await driver.wait(until.elementLocated(By.name('reply-body')), 5000)
    await replyBox.sendKeys(replyComment)
    await driver.findElement(By.xpath("//button[text()='Post reply']")).click()
    await driver.wait(until.elementLocated(By.xpath(`//p[contains(., '${replyComment}')]`)), 5000)

    bodyText = await driver.findElement(By.css('body')).getText()
    assert(bodyText.includes(rootComment), 'root comment should be visible')
    assert(bodyText.includes(replyComment), 'reply comment should be visible')

    const rootIndex = bodyText.indexOf(rootComment)
    const replyIndex = bodyText.indexOf(replyComment)
    assert(rootIndex < replyIndex, 'reply should render after its parent (nested)')

    console.log('e2e: threaded comments flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
