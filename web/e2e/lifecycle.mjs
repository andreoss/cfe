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

async function tick(driver, name) {
  const box = await driver.wait(until.elementLocated(By.name(name)), 10000)
  for (let attempt = 0; attempt < 10; attempt += 1) {
    if (await box.isSelected()) return
    await box.click()
    await new Promise((resolve) => setTimeout(resolve, 200))
  }
  throw new Error(`the ${name} checkbox never became checked`)
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

async function signIn(driver, username) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function run() {
  const author = await buildDriver()
  const mod = await buildDriver()
  const stranger = await buildDriver()
  const suffix = Date.now().toString(36)
  const authorName = `e2e_lc_a_${suffix}`
  const modName = `e2e_lc_m_${suffix}`
  const strangerName = `e2e_lc_s_${suffix}`
  const draftTitle = `Draft topic ${suffix}`
  const openTitle = `Open topic ${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await signIn(mod, modName)
    await register(author, authorName)
    await register(stranger, strangerName)

    await author.get(`${baseUrl}/s/general`)
    await openNewTopic(author)
    await author.findElement(By.name('title')).sendKeys(draftTitle)
    await author.findElement(By.name('body')).sendKeys('A body kept as a draft.')
    await tick(author, 'topic-draft')
    await clickWhenReady(author, By.xpath("//button[text()='Post']"), 'post control')
    await author.wait(
      async () => {
        const text = await mainText(author)
        return text.includes(draftTitle) || (await author.findElements(By.css('[role="alert"]'))).length > 0
      },
      10000,
      'creating the draft should report an outcome',
    )

    await author.get(`${baseUrl}/s/general`)
    await author.wait(
      async () => (await mainText(author)).includes(draftTitle),
      10000,
      'the author should see their own draft',
    )
    assert(
      (await mainText(author)).includes('Draft'),
      'the listing should mark it as a draft for its author',
    )

    await stranger.get(`${baseUrl}/s/general`)
    await stranger.wait(until.elementLocated(By.css('main')), 10000)
    assert(
      !(await mainText(stranger)).includes(draftTitle),
      'a stranger must not see a draft in the listing',
    )

    await author.get(`${baseUrl}/s/general`)
    await clickWhenReady(author, By.linkText(draftTitle), 'the draft should be openable')
    await author.wait(until.elementLocated(By.name('comment-body')), 10000)
    const draftUrl = await author.getCurrentUrl()

    await stranger.get(draftUrl)
    await stranger.wait(until.elementLocated(By.css('main')), 10000)
    assert(
      !(await mainText(stranger)).includes('A body kept as a draft.'),
      'a stranger must not read a draft by its link',
    )

    await clickWhenReady(author, By.xpath("//button[text()='Publish draft']"), 'publish control')
    await author.wait(
      async () => {
        const buttons = await author.findElements(By.xpath("//button[text()='Publish draft']"))
        return buttons.length === 0
      },
      10000,
      'publishing should retire the control',
    )

    await clickWhenReady(author, By.xpath("//button[text()='Mark resolved']"), 'resolve control')
    await author.wait(
      async () => (await mainText(author)).includes('Resolved'),
      10000,
      'the author should be able to mark the thread resolved',
    )

    await stranger.get(draftUrl)
    await stranger.wait(until.elementLocated(By.css('main')), 10000)
    const noResolve = await stranger.findElements(
      By.xpath("//button[text()='Mark resolved']"),
    )
    assert(noResolve.length === 0, 'a stranger must not be offered the resolve control')

    await mod.get(`${baseUrl}/s/general`)
    await openNewTopic(mod)
    await mod.findElement(By.name('title')).sendKeys(openTitle)
    await mod.findElement(By.name('body')).sendKeys('An ordinary topic for placement.')
    await clickWhenReady(mod, By.xpath("//button[text()='Post']"), 'post control')
    await publishTopic(openTitle)

    await mod.get(`${baseUrl}/s/general`)
    await clickWhenReady(mod, By.linkText(openTitle), 'the topic should be listed')
    await mod.wait(until.elementLocated(By.name('comment-body')), 10000)

    await clickWhenReady(mod, By.xpath("//summary[text()='Placement']"), 'the placement panel')
    await clickWhenReady(mod, By.xpath("//button[text()='Make sticky']"), 'the sticky control')
    await mod.wait(
      async () => (await mainText(mod)).includes('Unstick'),
      10000,
      'making it sticky should offer to unstick it',
    )

    await mod.get(`${baseUrl}/s/general`)
    await mod.wait(
      async () => (await mainText(mod)).includes('Sticky'),
      10000,
      'the listing should mark a sticky topic',
    )

    const listed = await mainText(mod)
    assert(
      listed.indexOf(openTitle) < listed.indexOf(draftTitle),
      'a sticky topic should come before the others',
    )

    console.log('e2e: content lifecycle flow passed')
  } finally {
    await author.quit()
    await mod.quit()
    await stranger.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
