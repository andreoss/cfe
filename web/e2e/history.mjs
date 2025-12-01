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
  const reader = await buildDriver()
  const suffix = Date.now().toString(36)
  const authorName = `e2e_hist_${suffix}`
  const title = `A subject with history ${suffix}`
  try {
    await register(author, authorName)

    await author.get(`${baseUrl}/s/general`)
    await author.wait(
      until.elementLocated(By.xpath("//button[text()='New topic']")),
      10000,
      'the new topic control should appear once posting is known to be allowed',
    )
    await author.findElement(By.xpath("//button[text()='New topic']")).click()
    await author.wait(until.elementLocated(By.name('title')), 10000)
    await author.findElement(By.name('title')).sendKeys(title)
    await author.findElement(By.name('body')).sendKeys('line one\nline two')
    await clickWhenReady(author, By.xpath("//button[text()='Post']"), 'the post control')
    await publishTopic(title)
    await author.get(`${baseUrl}/s/general`)
    await author.wait(until.elementLocated(By.linkText(title)), 10000)
    await author.findElement(By.linkText(title)).click()
    await author.wait(until.elementLocated(By.name('comment-body')), 10000)
    const topicUrl = await author.getCurrentUrl()
    const topicId = topicUrl.split('/t/')[1]

    const unedited = await mainText(author)
    assert(
      !unedited.includes('History'),
      `an unedited subject should not offer a history, saw: ${unedited}`,
    )

    await author.get(`${baseUrl}/t/${topicId}/history`)
    await author.wait(
      async () => (await mainText(author)).includes('No earlier versions.'),
      10000,
      'a subject with no edits should say so',
    )

    await author.get(topicUrl)
    await clickWhenReady(author, By.xpath("//button[text()='Edit topic']"), 'the edit control')
    const bodyField = await author.wait(until.elementLocated(By.name('edit-body')), 10000)
    await bodyField.clear()
    await bodyField.sendKeys('line one\nline two changed')
    await clickWhenReady(author, By.xpath("//button[text()='Save changes']"), 'the save control')
    await author.wait(
      async () => (await mainText(author)).includes('line two changed'),
      20000,
      'the edit should land',
    )

    await author.get(topicUrl)
    await clickWhenReady(author, By.linkText('History'), 'the history link')
    await author.wait(until.urlContains('/history'), 10000)
    await author.wait(
      async () => (await mainText(author)).includes(authorName),
      10000,
      'the earlier version should be listed with who wrote it',
    )
    const listed = await mainText(author)
    assert(listed.includes('History'), `the page should be headed, saw: ${listed}`)
    assert(listed.includes(authorName), `the version should name its editor, saw: ${listed}`)
    assert(
      !listed.includes('No earlier versions.'),
      'a subject with an edit should not read as unedited',
    )

    await clickWhenReady(author, By.xpath("//button[text()='What changed']"), 'the difference')
    await author.wait(
      until.elementLocated(By.css('.removed')),
      10000,
      'the difference should mark what went',
    )
    const kept = await author.findElements(By.css('.kept'))
    const removed = await author.findElements(By.css('.removed'))
    const added = await author.findElements(By.css('.added'))
    assert(kept.length >= 1, 'an unchanged line should read as kept')
    assert(removed.length >= 1, 'a replaced line should read as removed')
    assert(added.length >= 1, 'a replacing line should read as added')
    assert(
      (await removed[0].getText()).includes('line two'),
      `the removed line should be shown, saw: ${await removed[0].getText()}`,
    )
    assert(
      (await added[0].getText()).includes('line two changed'),
      `the added line should be shown, saw: ${await added[0].getText()}`,
    )

    await reader.get(`${baseUrl}/t/${topicId}/history`)
    await reader.wait(
      async () => (await mainText(reader)).includes(authorName),
      10000,
      'a history is public',
    )

    console.log('e2e: edit history flow passed')
  } finally {
    await author.quit()
    await reader.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
