import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot } from './support.mjs'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

const MONTHS = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
]

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

async function postTopic(driver, title) {
  await driver.get(`${baseUrl}/s/general`)
  await driver.wait(
    until.elementLocated(By.xpath("//button[text()='New topic']")),
    10000,
    'the new topic control should appear once posting is known to be allowed',
  )
  await driver.findElement(By.xpath("//button[text()='New topic']")).click()
  await driver.wait(until.elementLocated(By.name('title')), 10000)
  await driver.findElement(By.name('title')).sendKeys(title)
  await driver.findElement(By.name('body')).sendKeys('A body for the archive spec.')
  await clickWhenReady(driver, By.xpath("//button[text()='Post']"), 'the post control')
  await driver.wait(
    async () => (await mainText(driver)).includes(title),
    10000,
    'posting should land on the subject that was just written',
  )
}

async function run() {
  const mod = await buildDriver()
  const reader = await buildDriver()
  const anon = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_arch_m_${suffix}`
  const readerName = `e2e_arch_r_${suffix}`
  const published = `An archived subject ${suffix}`
  const queued = `A queued subject ${suffix}`
  const now = new Date()
  const label = `${MONTHS[now.getUTCMonth()]} ${now.getUTCFullYear()}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await register(reader, readerName)

    await postTopic(mod, published)

    await anon.get(`${baseUrl}/`)
    await clickWhenReady(anon, By.linkText('Archive'), 'the archive link should be offered')
    await anon.wait(until.urlIs(`${baseUrl}/archive`), 10000)
    await anon.wait(
      async () => (await mainText(anon)).includes(label),
      10000,
      'the current month should be listed once something is published',
    )
    const listed = await mainText(anon)
    assert(listed.includes('Archive'), `the page should be headed, saw: ${listed}`)
    assert(
      /\d+ subjects?\b/.test(listed),
      `each month should carry a count, saw: ${listed}`,
    )

    await clickWhenReady(anon, By.linkText(label), 'the month link')
    await anon.wait(
      async () => (await mainText(anon)).includes(published),
      10000,
      'the month should list what was posted in it',
    )
    const monthText = await mainText(anon)
    assert(monthText.includes(label), `the month page should be headed, saw: ${monthText}`)

    await clickWhenReady(anon, By.linkText(published), 'the subject link')
    await anon.wait(until.urlContains('/t/'), 10000)
    const opened = await mainText(anon)
    assert(opened.includes(published), `the subject should open from the archive, saw: ${opened}`)

    await postTopic(reader, queued)
    await anon.get(`${baseUrl}/archive`)
    await anon.wait(
      async () => (await mainText(anon)).includes(label),
      10000,
      'the archive should still list the month',
    )
    await clickWhenReady(anon, By.linkText(label), 'the month link')
    await anon.wait(
      async () => (await mainText(anon)).includes(published),
      10000,
      'the published subject should still be listed',
    )
    const withQueued = await mainText(anon)
    assert(
      !withQueued.includes(queued),
      `a subject still queued for a moderator is not archived, saw: ${withQueued}`,
    )

    await mod.get(`${baseUrl}/archive/${now.getUTCFullYear()}/${now.getUTCMonth() + 1}`)
    await mod.wait(
      async () => (await mainText(mod)).includes(published),
      10000,
      'a moderator should see the archive too',
    )
    const modSees = await mainText(mod)
    assert(
      !modSees.includes(queued),
      `the archive is published history for a moderator as well, saw: ${modSees}`,
    )

    await anon.get(`${baseUrl}/archive/1999/3`)
    await anon.wait(
      async () => (await mainText(anon)).includes('Nothing posted in this month.'),
      10000,
      'a month nobody posted in should say so',
    )

    console.log('e2e: archive flow passed')
  } finally {
    await mod.quit()
    await reader.quit()
    await anon.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
