import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot, publishTopic } from './support.mjs'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const apiUrl = process.env.API_URL ?? 'http://127.0.0.1:58080'
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

async function scoreOf(username) {
  const response = await fetch(`${apiUrl}/api/users/${encodeURIComponent(username)}`)
  assert(response.ok, `reading ${username} failed with ${response.status}`)
  return (await response.json()).score
}

async function postAndPublish(driver, title) {
  await driver.get(`${baseUrl}/s/general`)
  await driver.wait(
    until.elementLocated(By.xpath("//button[text()='New topic']")),
    10000,
    'the new topic control should appear once posting is known to be allowed',
  )
  await driver.findElement(By.xpath("//button[text()='New topic']")).click()
  await driver.wait(until.elementLocated(By.name('title')), 10000)
  await driver.findElement(By.name('title')).sendKeys(title)
  await driver.findElement(By.name('body')).sendKeys('A body for the penalty spec.')
  await clickWhenReady(driver, By.xpath("//button[text()='Post']"), 'the post control')
  await publishTopic(title)
  await driver.get(`${baseUrl}/s/general`)
  await driver.wait(until.elementLocated(By.linkText(title)), 10000)
  await driver.findElement(By.linkText(title)).click()
  await driver.wait(until.elementLocated(By.name('comment-body')), 10000)
  return driver.getCurrentUrl()
}

async function deleteWith(mod, url, penalty) {
  await mod.get(url)
  await clickWhenReady(mod, By.xpath("//button[text()='Delete topic']"), 'the delete control')
  const reason = await mod.wait(until.elementLocated(By.name('delete-reason')), 10000)
  await reason.clear()
  await reason.sendKeys('spam')
  const choice = await mod.wait(until.elementLocated(By.name('penalty')), 10000)
  await choice.findElement(By.css(`option[value='${penalty}']`)).click()
  await clickWhenReady(mod, By.xpath("//button[text()='Confirm delete']"), 'the confirm control')
}

async function run() {
  const mod = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_pen_m_${suffix}`
  const authors = []
  const drivers = []
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)

    for (const which of ['a', 'b', 'c']) {
      const driver = await buildDriver()
      drivers.push(driver)
      const name = `e2e_pen_${which}_${suffix}`
      await register(driver, name)
      const url = await postAndPublish(driver, `Subject ${which} ${suffix}`)
      authors.push({ name, url })
      assert((await scoreOf(name)).valueOf() === 0, `${name} should start at nothing`)
    }

    await mod.get(authors[0].url)
    await clickWhenReady(mod, By.xpath("//button[text()='Delete topic']"), 'the delete control')
    await mod.wait(until.elementLocated(By.name('penalty')), 10000)
    const preset = await mod.findElement(By.name('penalty')).getAttribute('value')
    assert(preset === '-10', `the penalty should start at the customary one, was: ${preset}`)

    const reason = await mod.findElement(By.name('delete-reason'))
    await reason.sendKeys('spam')
    await clickWhenReady(mod, By.xpath("//button[text()='Confirm delete']"), 'the confirm control')
    await mod.wait(
      async () => (await scoreOf(authors[0].name)) === -10,
      10000,
      'leaving the penalty alone should cost the author the customary amount',
    )

    await deleteWith(mod, authors[1].url, '-30')
    await mod.wait(
      async () => (await scoreOf(authors[1].name)) === -30,
      10000,
      'a harsher penalty should cost the author more',
    )

    await deleteWith(mod, authors[2].url, '0')
    await mod.get(`${baseUrl}/s/general`)
    await mod.wait(
      async () =>
        (await mod.findElements(By.linkText(`Subject c ${suffix}`))).length === 0 &&
        (await mainText(mod)).length > 0,
      10000,
      'a deleted subject should leave the section listing',
    )
    assert(
      (await scoreOf(authors[2].name)) === 0,
      'choosing no penalty should leave the author untouched',
    )

    const stranger = await buildDriver()
    drivers.push(stranger)
    await stranger.get(authors[1].url)
    await stranger.wait(until.elementLocated(By.css('main')), 10000)
    const strangerSees = await mainText(stranger)
    assert(
      !strangerSees.includes('penalty'),
      `a stranger is offered no penalty control, saw: ${strangerSees}`,
    )

    console.log('e2e: deletion penalty flow passed')
  } finally {
    await mod.quit()
    for (const driver of drivers) await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
