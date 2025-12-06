import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot, publishTopic } from './support.mjs'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

const PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGP4DwABAQEAGdiK2wAAAABJRU5ErkJggg=='

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function writeFixtures() {
  const dir = mkdtempSync(join(tmpdir(), 'e2e-images-'))
  const png = join(dir, 'picture.png')
  writeFileSync(png, Buffer.from(PNG_BASE64, 'base64'))
  const sneaky = join(dir, 'sneaky.png')
  writeFileSync(sneaky, '<svg onload=alert(1)></svg>')
  return { png, sneaky }
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

async function run() {
  const author = await buildDriver()
  const stranger = await buildDriver()
  const suffix = Date.now().toString(36)
  const authorName = `e2e_img_a_${suffix}`
  const strangerName = `e2e_img_s_${suffix}`
  const title = `A subject with images ${suffix}`
  const { png, sneaky } = writeFixtures()
  try {
    await register(author, authorName)
    await register(stranger, strangerName)
    await promoteViaRoot(authorName)

    await author.get(`${baseUrl}/s/general`)
    await author.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'the new topic control should appear once posting is known to be allowed',
    )
    await author.findElement(By.xpath("//button[text()='New topic']")).click()
    await author.wait(until.elementLocated(By.name('title')), 10000)
    await author.findElement(By.name('title')).sendKeys(title)
    await author.findElement(By.name('body')).sendKeys('A body for the images spec.')
    await clickWhenReady(author, By.xpath("//button[text()='Post']"), 'the post control')
    await publishTopic(title)
    await author.get(`${baseUrl}/s/general`)
    await author.wait(until.elementLocated(By.linkText(title)), 10000)
    await author.findElement(By.linkText(title)).click()
    await author.wait(until.elementLocated(By.name('comment-body')), 10000)
    const topicUrl = await author.getCurrentUrl()

    const before = await author.findElements(By.css('img.attachment'))
    assert(before.length === 0, 'a subject starts with no images')

    await author.wait(until.elementLocated(By.name('image-file')), 10000)
    await author.findElement(By.name('image-file')).sendKeys(png)
    await clickWhenReady(author, By.xpath("//button[text()='Attach image']"), 'the attach control')
    await author.wait(
      until.elementLocated(By.css('img.attachment')),
      15000,
      'the attached image should appear without a reload',
    )

    const attached = await author.findElement(By.css('img.attachment'))
    const src = await attached.getAttribute('src')
    assert(src.includes('/images/'), `the image should be served from its own url, was: ${src}`)
    await author.wait(
      async () =>
        author.executeScript(
          'const i = document.querySelector("img.attachment"); return i !== null && i.complete && i.naturalWidth > 0',
        ),
      15000,
      'the image should actually load',
    )

    await stranger.get(topicUrl)
    await stranger.wait(
      until.elementLocated(By.css('img.attachment')),
      15000,
      'anybody may see an attached image',
    )
    const strangerSees = await mainText(stranger)
    assert(
      (await stranger.findElements(By.name('image-file'))).length === 0,
      `a stranger is offered no way to attach, saw: ${strangerSees}`,
    )
    assert(
      (await stranger.findElements(By.xpath("//button[text()='Remove image']"))).length === 0,
      'a stranger is offered no way to remove one',
    )

    await author.get(topicUrl)
    await author.wait(until.elementLocated(By.name('image-file')), 10000)
    await author.findElement(By.name('image-file')).sendKeys(sneaky)
    await clickWhenReady(author, By.xpath("//button[text()='Attach image']"), 'the attach control')
    await author.wait(
      async () => (await mainText(author)).includes('unsupported image'),
      15000,
      'something that is not an image should be refused, and say so',
    )
    const stillOne = await author.findElements(By.css('img.attachment'))
    assert(stillOne.length === 1, 'a refused upload should not attach anything')

    await clickWhenReady(author, By.xpath("//button[text()='Remove image']"), 'the remove control')
    await author.wait(
      async () => (await author.findElements(By.css('img.attachment'))).length === 0,
      15000,
      'a removed image should go without a reload',
    )

    await author.get(topicUrl)
    await author.wait(until.elementLocated(By.name('image-file')), 10000)
    const gone = await author.findElements(By.css('img.attachment'))
    assert(gone.length === 0, 'a removed image should stay gone')

    console.log('e2e: images flow passed')
  } finally {
    await author.quit()
    await stranger.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
