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

async function mainText(driver) {
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      return (await driver.findElement(By.css('main')).getText()).replace(/\s+/g, ' ')
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function clickWhenReady(driver, locator, message) {
  await driver.wait(until.elementLocated(locator), 15000, message)
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
  await driver.wait(until.elementLocated(By.name('username')), 15000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 15000)
}

async function unread(driver) {
  const text = await driver.findElement(By.css('nav')).getText()
  const found = /Notifications \((\d+)\)/.exec(text)
  return found === null ? 0 : Number(found[1])
}

async function comment(driver, topicUrl, body) {
  await driver.get(topicUrl)
  await driver.wait(until.elementLocated(By.name('comment-body')), 15000)
  await driver.findElement(By.name('comment-body')).sendKeys(body)
  await clickWhenReady(driver, By.xpath("//button[text()='Post comment']"), 'the comment control')
  await driver.wait(
    async () => (await mainText(driver)).includes(body),
    15000,
    `the comment "${body}" should appear for whoever wrote it`,
  )
}

async function run() {
  const reader = await buildDriver()
  const talker = await buildDriver()
  const suffix = Date.now().toString(36)
  const readerName = `e2e_live_r_${suffix}`
  const talkerName = `e2e_live_t_${suffix}`
  const title = `A subject that moves ${suffix}`
  const arrival = `A remark that arrives while reading ${suffix}`
  try {
    await register(reader, readerName)
    await register(talker, talkerName)

    await reader.get(`${baseUrl}/s/general`)
    await reader.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      15000,
      'the new topic control should appear once posting is known to be allowed',
    )
    await reader.findElement(By.xpath("//button[text()='New topic']")).click()
    await reader.wait(until.elementLocated(By.name('title')), 15000)
    await reader.findElement(By.name('title')).sendKeys(title)
    await reader.findElement(By.name('body')).sendKeys('A body for the live spec.')
    await clickWhenReady(reader, By.xpath("//button[text()='Post']"), 'the post control')
    await publishTopic(title)

    await reader.get(`${baseUrl}/s/general`)
    await reader.wait(until.elementLocated(By.linkText(title)), 15000)
    await reader.findElement(By.linkText(title)).click()
    await reader.wait(until.elementLocated(By.name('comment-body')), 15000)
    const topicUrl = await reader.getCurrentUrl()

    await reader.wait(
      async () => (await mainText(reader)).includes('No comments yet'),
      15000,
      'the subject should be read as empty before anybody speaks',
    )
    assert(
      (await reader.findElements(By.css('button.waiting'))).length === 0,
      'nothing is waiting on a quiet subject',
    )

    await comment(talker, topicUrl, arrival)

    await reader.wait(
      until.elementLocated(By.css('button.waiting')),
      45000,
      'a remark posted by somebody else should be offered without a reload',
    )
    const offer = await reader.findElement(By.css('button.waiting')).getText()
    assert(
      offer === 'Show 1 new comment',
      `one remark should be offered in the singular, said: ${offer}`,
    )

    const beforeShowing = await mainText(reader)
    assert(
      !beforeShowing.includes(arrival),
      'a remark should not be pushed under a reader who has not asked for it',
    )

    await clickWhenReady(reader, By.css('button.waiting'), 'the offer control')
    await reader.wait(
      async () => (await mainText(reader)).includes(arrival),
      15000,
      'accepting the offer should bring the remark in',
    )
    await reader.wait(
      async () => (await reader.findElements(By.css('button.waiting'))).length === 0,
      15000,
      'the offer should go once it has been taken',
    )

    const second = `A second remark ${suffix}`
    const third = `A third remark ${suffix}`
    await comment(talker, topicUrl, second)
    await comment(talker, topicUrl, third)
    await reader.wait(
      async () => {
        const found = await reader.findElements(By.css('button.waiting'))
        if (found.length === 0) return false
        return (await found[0].getText()) === 'Show 2 new comments'
      },
      45000,
      'two remarks should be counted, and read as plural',
    )

    await reader.get(topicUrl)
    await reader.wait(
      async () => (await mainText(reader)).includes(third),
      15000,
      'a reload should bring everything in and leave nothing waiting',
    )
    assert(
      (await reader.findElements(By.css('button.waiting'))).length === 0,
      'a reader who has just loaded the page is not behind',
    )

    await clickWhenReady(reader, By.xpath("//button[text()='Watch topic']"), 'the watch control')
    await reader.wait(
      until.elementLocated(By.xpath("//button[text()='Stop watching']")),
      15000,
      'watching should be acknowledged',
    )
    const before = await unread(reader)
    await comment(talker, topicUrl, `A remark for the count ${suffix}`)
    await reader.wait(
      async () => (await unread(reader)) > before,
      45000,
      `the unread count should climb on its own from ${before}, without a reload`,
    )

    console.log('e2e: live comments flow passed')
  } finally {
    await reader.quit()
    await talker.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
