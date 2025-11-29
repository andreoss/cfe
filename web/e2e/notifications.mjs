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

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 5000)
}

async function navText(driver) {
  await driver.get(`${baseUrl}/`)
  await driver.wait(until.elementLocated(By.css('nav')), 5000)
  await driver.sleep(500)
  return driver.findElement(By.css('nav')).getText()
}

async function run() {
  const author = await buildDriver()
  const suffix = Date.now().toString(36)
  const authorName = `e2e_notif_a_${suffix}`
  const readerName = `e2e_notif_b_${suffix}`
  const topicTitle = `Notified subject ${suffix}`
  let reader
  try {
    await register(author, authorName)

    await author.wait(until.elementLocated(By.linkText('General')), 10000)


    await author.findElement(By.linkText('General')).click()
    await author.wait(until.urlIs(`${baseUrl}/s/general`), 5000)
    await author.findElement(By.xpath("//button[text()='New topic']")).click()
    await author.wait(until.elementLocated(By.name('title')), 10000)
    await author.findElement(By.name('title')).sendKeys(topicTitle)
    await author.findElement(By.name('body')).sendKeys('Body for the notifications e2e spec.')
    await author.findElement(By.xpath("//button[text()='Post']")).click()
    await publishTopic(topicTitle)
    await author.get(`${baseUrl}/s/general`)
    await author.wait(until.elementLocated(By.linkText(topicTitle)), 5000)
    await author.findElement(By.linkText(topicTitle)).click()
    await author.wait(until.elementLocated(By.name('comment-body')), 5000)
    const topicUrl = await author.getCurrentUrl()

    let nav = await navText(author)
    assert(nav.includes('Notifications'), 'a signed-in user should see the notifications link')
    assert(!nav.includes('Notifications ('), 'a new user should have no unread count')

    await author.get(topicUrl)
    await author.wait(until.elementLocated(By.name('comment-body')), 5000)
    await author.findElement(By.name('comment-body')).sendKeys('A note to myself')
    await author.findElement(By.xpath("//button[text()='Post comment']")).click()
    await author.wait(until.elementLocated(By.xpath("//p[contains(., 'A note to myself')]")), 5000)

    nav = await navText(author)
    assert(!nav.includes('Notifications ('), 'commenting on your own topic must not notify you')

    reader = await buildDriver()
    await register(reader, readerName)
    await reader.get(topicUrl)
    await reader.wait(until.elementLocated(By.name('comment-body')), 5000)
    await reader.findElement(By.name('comment-body')).sendKeys('A comment from a reader')
    await reader.findElement(By.xpath("//button[text()='Post comment']")).click()
    await reader.wait(
      until.elementLocated(By.xpath("//p[contains(., 'A comment from a reader')]")),
      5000,
    )

    nav = await navText(author)
    assert(nav.includes('Notifications (1)'), `author should have one unread, nav was: ${nav}`)

    const readerNav = await navText(reader)
    assert(
      !readerNav.includes('Notifications ('),
      'the commenter must not be notified about their own comment',
    )

    await author.get(`${baseUrl}/notifications`)
    await author.wait(until.elementLocated(By.xpath("//a[contains(., '" + topicTitle + "')]")), 5000)
    let text = await author.findElement(By.css('body')).getText()
    assert(text.includes(`${readerName} replied`), 'the notification should name the actor')
    assert(text.includes(topicTitle), 'the notification should link the topic by title')

    await author.findElement(By.xpath("//button[normalize-space(text())='Mark read']")).click()
    await author.sleep(800)
    text = await author.findElement(By.css('body')).getText()
    assert(!text.includes('Mark read'), 'a read notification should lose its mark-read control')

    nav = await navText(author)
    assert(!nav.includes('Notifications ('), 'the unread count should clear once read')

    await author.get(`${baseUrl}/notifications`)
    await author.wait(until.elementLocated(By.css('body')), 5000)
    await author.sleep(500)
    text = await author.findElement(By.css('body')).getText()
    assert(text.includes(topicTitle), 'a read notification should still be listed')
    assert(!text.includes('Mark read'), 'the read state should survive a reload')

    const readerText = await (async () => {
      await reader.get(`${baseUrl}/notifications`)
      await reader.wait(until.elementLocated(By.css('body')), 5000)
      await reader.sleep(500)
      return reader.findElement(By.css('body')).getText()
    })()
    assert(
      readerText.includes('No notifications.'),
      'the reader should not see another user notifications',
    )

    console.log('e2e: notifications flow passed')
  } finally {
    await author.quit()
    if (reader) await reader.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
