import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot, publishTopic } from './support.mjs'

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
      return await driver.findElement(By.css('main')).getText()
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function clickWhenReady(driver, locator, message) {
  await driver.wait(until.elementLocated(locator), 10000, message)
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

async function openNewTopic(driver) {
  let last
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      await clickWhenReady(
        driver,
        By.xpath("//button[text()='New topic']"),
        'the new topic control should be present',
      )
      await driver.wait(until.elementLocated(By.name('title')), 2000)
      return
    } catch (err) {
      last = err
    }
  }
  throw last
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function comment(driver, topicUrl, body) {
  await driver.get(topicUrl)
  const box = await driver.wait(until.elementLocated(By.name('comment-body')), 10000)
  await box.clear()
  await box.sendKeys(body)
  await clickWhenReady(driver, By.xpath("//button[text()='Post comment']"), 'post comment control')
  await driver.wait(
    async () => (await mainText(driver)).includes(body),
    10000,
    'the comment should land',
  )
}

async function run() {
  const author = await buildDriver()
  const watcher = await buildDriver()
  const stranger = await buildDriver()
  const suffix = Date.now().toString(36)
  const authorName = `e2e_wa_a_${suffix}`
  const watcherName = `e2e_wa_w_${suffix}`
  const strangerName = `e2e_wa_s_${suffix}`
  const title = `Watched subject ${suffix}`
  try {
    await register(author, authorName)
    await promoteViaRoot(authorName)
    await register(watcher, watcherName)
    await register(stranger, strangerName)

    await author.get(`${baseUrl}/s/general`)
    await openNewTopic(author)
    await author.findElement(By.name('title')).sendKeys(title)
    await author.findElement(By.name('body')).sendKeys('A topic somebody will watch.')
    await clickWhenReady(author, By.xpath("//button[text()='Post']"), 'post control')
    await author.wait(
      async () => (await mainText(author)).includes(title),
      20000,
      'the topic should be created',
    )
    await publishTopic(title)

    await watcher.get(`${baseUrl}/s/general`)
    await clickWhenReady(watcher, By.linkText(title), 'the topic should be listed')
    await watcher.wait(until.elementLocated(By.name('comment-body')), 10000)
    const topicUrl = await watcher.getCurrentUrl()

    await watcher.get(`${baseUrl}/watched`)
    await watcher.wait(
      async () => (await mainText(watcher)).includes('You are not watching anything.'),
      10000,
      'the watched list should start empty',
    )

    await watcher.get(topicUrl)
    await clickWhenReady(watcher, By.xpath("//button[text()='Watch topic']"), 'the watch control')
    await watcher.wait(
      async () =>
        (await watcher.findElements(By.xpath("//button[text()='Stop watching']"))).length > 0,
      10000,
      'watching should offer to stop',
    )

    await watcher.get(`${baseUrl}/watched`)
    await watcher.wait(
      async () => (await mainText(watcher)).includes(title),
      10000,
      'the watched list should name the topic',
    )

    await stranger.get(`${baseUrl}/watched`)
    await stranger.wait(
      async () => (await mainText(stranger)).includes('You are not watching anything.'),
      10000,
      'a stranger watches nothing',
    )

    await comment(stranger, topicUrl, `A passing remark ${suffix}`)

    await watcher.get(`${baseUrl}/notifications`)
    await watcher.wait(
      async () => (await mainText(watcher)).includes('Watched topic'),
      10000,
      'the watcher should be told, as a watch',
    )

    await author.get(`${baseUrl}/notifications`)
    await author.wait(
      async () => (await mainText(author)).includes('Reply'),
      10000,
      'the author should be told, as a reply',
    )
    const authorText = await mainText(author)
    assert(
      !authorText.includes('Watched topic'),
      `the author should not also be told as a watcher, saw: ${authorText}`,
    )

    await watcher.get(topicUrl)
    await clickWhenReady(watcher, By.xpath("//button[text()='Stop watching']"), 'the stop control')
    await watcher.wait(
      async () =>
        (await watcher.findElements(By.xpath("//button[text()='Watch topic']"))).length > 0,
      10000,
      'stopping should offer to watch again',
    )

    await watcher.get(`${baseUrl}/watched`)
    await watcher.wait(
      async () => (await mainText(watcher)).includes('You are not watching anything.'),
      10000,
      'the watched list should be empty again',
    )

    await stranger.get(`${baseUrl}/`)
    await clickWhenReady(
      stranger,
      By.xpath("//button[normalize-space(.)='Sign out']"),
      'the sign-out control',
    )
    await stranger.wait(
      async () => (await stranger.findElement(By.css('nav')).getText()).includes('Sign in'),
      10000,
      'signing out should show the sign-in link',
    )
    await stranger.get(topicUrl)
    await stranger.wait(until.elementLocated(By.css('main')), 10000)
    const anonymous = await stranger.findElements(By.xpath("//button[text()='Watch topic']"))
    assert(anonymous.length === 0, 'an anonymous visitor must not see the watch control')

    console.log('e2e: watching flow passed')
  } finally {
    await author.quit()
    await watcher.quit()
    await stranger.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
