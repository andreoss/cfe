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
      return (await driver.findElement(By.css('main')).getText()).replace(/\s+/g, ' ')
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

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 10000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function fill(driver, name, value) {
  const field = await driver.wait(until.elementLocated(By.name(name)), 10000)
  await field.clear()
  await field.sendKeys(value)
}

async function run() {
  const mod = await buildDriver()
  const reader = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_tag_m_${suffix}`
  const readerName = `e2e_tag_r_${suffix}`
  const tag = `tg${suffix}`
  const synonym = `tgalt${suffix}`
  const title = `A tagged subject ${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await register(reader, readerName)

    await mod.get(`${baseUrl}/s/general`)
    await mod.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'the new topic control should appear once posting is known to be allowed',
    )
    await mod.findElement(By.xpath("//button[text()='New topic']")).click()
    await mod.wait(until.elementLocated(By.name('title')), 10000)
    await mod.findElement(By.name('title')).sendKeys(title)
    await mod.findElement(By.name('body')).sendKeys('A body for the tag spec.')
    await mod.findElement(By.name('tags')).sendKeys(tag)
    await clickWhenReady(mod, By.xpath("//button[text()='Post']"), 'the post control')
    await publishTopic(title)

    await mod.get(`${baseUrl}/tag/${tag}`)
    await mod.wait(
      async () => (await mainText(mod)).includes(title),
      15000,
      'the tag should list what carries it',
    )

    await fill(mod, 'tag-description', 'about the tag spec')
    await clickWhenReady(mod, By.xpath("//button[text()='Save description']"), 'the save control')
    await mod.wait(
      async () => (await mainText(mod)).includes('about the tag spec'),
      10000,
      'a described tag should say what it is for',
    )

    await reader.get(`${baseUrl}/tag/${tag}`)
    await reader.wait(
      async () => (await mainText(reader)).includes('about the tag spec'),
      10000,
      'anybody may read what a tag is for',
    )
    assert(
      (await reader.findElements(By.name('tag-description'))).length === 0,
      'a reader is offered no way to describe a tag',
    )

    await mod.get(`${baseUrl}/tag/${synonym}`)
    await fill(mod, 'tag-means', tag)
    await clickWhenReady(mod, By.xpath("//button[text()='Set synonym']"), 'the synonym control')
    await mod.wait(
      async () => (await mainText(mod)).includes('Means '),
      10000,
      'a synonym should say what it means',
    )
    await mod.wait(
      until.elementLocated(By.linkText(tag)),
      10000,
      'a synonym should link to what it means',
    )

    await mod.get(`${baseUrl}/tag/${synonym}`)
    await mod.wait(
      async () => (await mainText(mod)).includes(title),
      15000,
      'a synonym shows what the tag it means holds',
    )

    await reader.get(`${baseUrl}/followed-tags`)
    await reader.wait(
      async () => (await mainText(reader)).includes('You follow no tags.'),
      10000,
      'somebody following nothing should be told so',
    )

    await reader.get(`${baseUrl}/tag/${synonym}`)
    await clickWhenReady(reader, By.xpath("//button[text()='Follow tag']"), 'the follow control')
    await reader.wait(
      until.elementLocated(By.xpath("//button[text()='Unfollow tag']")),
      10000,
      'following should turn the control around',
    )

    await reader.get(`${baseUrl}/followed-tags`)
    await reader.wait(
      async () => (await mainText(reader)).includes(tag),
      10000,
      'following a synonym should follow what it means',
    )
    const followed = await mainText(reader)
    assert(
      !followed.includes('You follow no tags.'),
      'somebody following a tag should not read as following none',
    )

    await reader.get(`${baseUrl}/tag/${tag}`)
    await clickWhenReady(reader, By.xpath("//button[text()='Unfollow tag']"), 'the unfollow control')
    await reader.wait(
      until.elementLocated(By.xpath("//button[text()='Follow tag']")),
      10000,
      'letting a tag go should turn the control back',
    )

    await reader.get(`${baseUrl}/followed-tags`)
    await reader.wait(
      async () => (await mainText(reader)).includes('You follow no tags.'),
      10000,
      'letting the last tag go should read as following none',
    )

    console.log('e2e: tag depth flow passed')
  } finally {
    await mod.quit()
    await reader.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
