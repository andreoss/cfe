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

async function signIn(driver, username) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 10000)
}

async function setRestriction(driver, topicUrl, label) {
  await driver.get(topicUrl)
  await clickWhenReady(
    driver,
    By.xpath("//summary[text()='Comment restrictions']"),
    'a moderator should see the comment restrictions panel',
  )
  const select = await driver.wait(until.elementLocated(By.name('postscore')), 10000)
  await select.findElement(By.xpath(`./option[normalize-space(.)='${label}']`)).click()
  await driver.wait(
    async () => {
      const value = await driver.findElement(By.name('postscore')).getAttribute('value')
      return value !== ''
    },
    10000,
    'the restriction should be applied',
  )
}

async function tryComment(driver, topicUrl, body) {
  await driver.get(topicUrl)
  const box = await driver.wait(until.elementLocated(By.name('comment-body')), 10000)
  await box.clear()
  await box.sendKeys(body)
  await clickWhenReady(
    driver,
    By.xpath("//button[text()='Post comment']"),
    'the post control should be present',
  )
  await driver.wait(
    async () => {
      const text = await mainText(driver)
      return text.includes(body) || text.includes('not allowed to comment')
    },
    10000,
    'posting should either land or be refused',
  )
  return mainText(driver)
}

async function run() {
  const mod = await buildDriver()
  const author = await buildDriver()
  const stranger = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_pm_m_${suffix}`
  const authorName = `e2e_pm_a_${suffix}`
  const strangerName = `e2e_pm_s_${suffix}`
  const topicTitle = `Restricted topic ${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await signIn(mod, modName)
    await register(author, authorName)
    await register(stranger, strangerName)

    await author.get(`${baseUrl}/s/general`)
    await openNewTopic(author)
    await author.findElement(By.name('title')).sendKeys(topicTitle)
    await author.findElement(By.name('body')).sendKeys('A topic whose comments get restricted.')
    await clickWhenReady(author, By.xpath("//button[text()='Post']"), 'post control')
    await publishTopic(topicTitle)

    await author.get(`${baseUrl}/s/general`)
    await clickWhenReady(author, By.linkText(topicTitle), 'the published topic should be listed')
    await author.wait(until.elementLocated(By.name('comment-body')), 10000)
    const topicUrl = await author.getCurrentUrl()

    let text = await tryComment(stranger, topicUrl, `Open comment ${suffix}`)
    assert(
      text.includes(`Open comment ${suffix}`),
      `anyone should be able to comment while unrestricted, saw: ${text}`,
    )

    await setRestriction(mod, topicUrl, 'Score 50')

    text = await tryComment(stranger, topicUrl, `Blocked comment ${suffix}`)
    assert(
      text.includes('not allowed to comment'),
      `a user below the bar should be refused, saw: ${text}`,
    )
    assert(
      !text.includes(`Blocked comment ${suffix}`),
      'the refused comment must not appear on the topic',
    )

    text = await tryComment(author, topicUrl, `Author comment ${suffix}`)
    assert(
      text.includes(`Author comment ${suffix}`),
      `the author should still be able to comment, saw: ${text}`,
    )

    text = await tryComment(mod, topicUrl, `Moderator comment ${suffix}`)
    assert(
      text.includes(`Moderator comment ${suffix}`),
      `a moderator should still be able to comment, saw: ${text}`,
    )

    await setRestriction(mod, topicUrl, 'Moderators only')

    text = await tryComment(author, topicUrl, `Author again ${suffix}`)
    assert(
      text.includes('not allowed to comment'),
      `moderators-only should exclude even the author, saw: ${text}`,
    )

    await setRestriction(mod, topicUrl, 'Anyone')

    text = await tryComment(stranger, topicUrl, `Reopened comment ${suffix}`)
    assert(
      text.includes(`Reopened comment ${suffix}`),
      `lifting the restriction should let anyone comment again, saw: ${text}`,
    )

    await stranger.get(topicUrl)
    const noPanel = await stranger.findElements(
      By.xpath("//summary[text()='Comment restrictions']"),
    )
    assert(noPanel.length === 0, 'a plain user must not see the restriction control')

    console.log('e2e: reputation permissions flow passed')
  } finally {
    await mod.quit()
    await author.quit()
    await stranger.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
