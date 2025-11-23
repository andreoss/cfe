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

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function signIn(driver, username) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
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

async function postTopic(driver, title) {
  await driver.get(`${baseUrl}/s/general`)
  await openNewTopic(driver)
  await driver.findElement(By.name('title')).sendKeys(title)
  await driver.findElement(By.name('body')).sendKeys('A body for the address spec.')
  await clickWhenReady(driver, By.xpath("//button[text()='Post']"), 'post control')
  await driver.wait(
    async () => (await mainText(driver)).includes(title),
    10000,
    'the topic should be created',
  )
}

async function investigate(driver) {
  await driver.get(`${baseUrl}/addresses`)
  const input = await driver.wait(until.elementLocated(By.name('investigate-addr')), 10000)
  await input.clear()
  await input.sendKeys('127.0.0.1')
  await clickWhenReady(driver, By.xpath("//button[text()='Investigate']"), 'investigate control')
  await driver.wait(
    async () => {
      const text = await mainText(driver)
      return text.includes('Remove posts') || text.includes('No posts from that address.')
    },
    10000,
    'the listing should come back',
  )
  return mainText(driver)
}

async function run() {
  const mod = await buildDriver()
  const author = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_ad_m_${suffix}`
  const authorName = `e2e_ad_a_${suffix}`
  const title = `Address topic ${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await signIn(mod, modName)
    await register(author, authorName)

    await postTopic(author, title)
    await publishTopic(title)

    await author.get(`${baseUrl}/addresses`)
    await author.wait(
      async () => (await mainText(author)).includes('Only a moderator can investigate an address.'),
      10000,
      'a plain user must not investigate',
    )
    const noLink = await author.findElements(By.linkText('Addresses'))
    assert(noLink.length === 0, 'a plain user must not see the addresses link')

    const listing = await investigate(mod)
    assert(
      listing.includes(authorName),
      `the listing should name who posted from the address, saw: ${listing}`,
    )

    const hours = await mod.findElement(By.name('remove-hours'))
    await hours.clear()
    await hours.sendKeys('1')
    const reason = await mod.findElement(By.name('remove-reason'))
    await reason.clear()
    await reason.sendKeys('a flood')
    await clickWhenReady(mod, By.xpath("//button[text()='Remove posts']"), 'remove control')
    await mod.wait(
      async () => /Removed \d+ posts\./.test(await mainText(mod)),
      10000,
      'removal should report how many it took',
    )

    await author.get(`${baseUrl}/s/general`)
    await author.wait(until.elementLocated(By.css('main')), 10000)
    assert(
      !(await mainText(author)).includes(title),
      'a removed topic must not stay in the section listing',
    )

    console.log('e2e: address investigation flow passed')
  } finally {
    await mod.quit()
    await author.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
