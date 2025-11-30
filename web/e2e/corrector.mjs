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

async function setRole(driver, username, role) {
  await driver.get(`${baseUrl}/u/${username}`)
  const select = await driver.wait(until.elementLocated(By.name('user-role')), 10000)
  await select.findElement(By.xpath(`./option[@value='${role}']`)).click()
  await clickWhenReady(driver, By.xpath("//button[text()='Set role']"), 'the set role control')
  await driver.wait(
    async () => (await mainText(driver)).includes('Role updated.'),
    10000,
    'setting a role should report success',
  )
}

async function run() {
  const mod = await buildDriver()
  const corrector = await buildDriver()
  const author = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_co_m_${suffix}`
  const correctorName = `e2e_co_c_${suffix}`
  const authorName = `e2e_co_a_${suffix}`
  const title = `Speling ${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await signIn(mod, modName)
    await register(corrector, correctorName)
    await register(author, authorName)

    await author.get(`${baseUrl}/s/general`)
    await openNewTopic(author)
    await author.findElement(By.name('title')).sendKeys(title)
    await author.findElement(By.name('body')).sendKeys('A body with a typo to correct.')
    await clickWhenReady(author, By.xpath("//button[text()='Post']"), 'post control')
    await author.wait(
      async () => (await mainText(author)).includes(title),
      20000,
      'the topic should be created',
    )
    await publishTopic(title)

    await corrector.get(`${baseUrl}/s/general`)
    await clickWhenReady(corrector, By.linkText(title), 'the topic should be listed')
    await corrector.wait(until.elementLocated(By.name('comment-body')), 10000)
    const topicUrl = await corrector.getCurrentUrl()

    let controls = await corrector.findElements(By.xpath("//button[text()='Edit topic']"))
    assert(controls.length === 0, 'a reader must not be offered the edit control')

    await corrector.get(`${baseUrl}/u/${correctorName}`)
    await corrector.wait(until.elementLocated(By.css('main')), 10000)
    const noRoleControl = await corrector.findElements(By.name('user-role'))
    assert(noRoleControl.length === 0, 'a reader must not be offered the role control')

    await setRole(mod, correctorName, 'corrector')

    await mod.get(`${baseUrl}/u/${modName}`)
    await mod.wait(until.elementLocated(By.css('main')), 10000)
    const ownControl = await mod.findElements(By.name('user-role'))
    assert(ownControl.length === 0, 'a moderator must not set their own role')

    await corrector.get(topicUrl)
    await corrector.wait(
      async () =>
        (await corrector.findElements(By.xpath("//button[text()='Edit topic']"))).length > 0,
      10000,
      'a corrector should be offered the edit control',
    )

    const noDelete = await corrector.findElements(By.xpath("//button[text()='Delete topic']"))
    assert(noDelete.length === 0, 'a corrector must not be offered the delete control')

    await clickWhenReady(corrector, By.xpath("//button[text()='Edit topic']"), 'edit control')
    const titleInput = await corrector.wait(until.elementLocated(By.name('edit-title')), 10000)
    await titleInput.clear()
    await titleInput.sendKeys(`Spelling ${suffix}`)
    await clickWhenReady(corrector, By.xpath("//button[text()='Save changes']"), 'save control')
    await corrector.wait(
      async () => (await mainText(corrector)).includes(`Spelling ${suffix}`),
      10000,
      'the corrector should be able to fix the wording',
    )

    await setRole(mod, correctorName, 'user')
    await corrector.get(topicUrl)
    await corrector.wait(until.elementLocated(By.css('main')), 10000)
    controls = await corrector.findElements(By.xpath("//button[text()='Edit topic']"))
    assert(controls.length === 0, 'revoking the role should retire the edit control')

    console.log('e2e: corrector flow passed')
  } finally {
    await mod.quit()
    await corrector.quit()
    await author.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
